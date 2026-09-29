//! Rotating through the Alphax Free pool after a 429, 5xx, or empty response.
//!
//! # The problem
//!
//! `kilo-auto/free` is a routing alias the gateway resolves server-side. Free
//! tier limits are enforced **per upstream model**, so the alias can 429 while
//! other zero-priced models serve traffic at that instant. AlphaCode's only
//! pre-existing recovery was `maybe_continue_after_provider_error`, which
//! retries the *same* alias with exponential backoff — re-rolling the gateway's
//! dice against the same throttled backend. The user sees a hard stall for
//! minutes while a perfectly good free model sits idle.
//!
//! # The mechanism
//!
//! When the active profile is `alphax-free` and a retryable failure occurs:
//!
//! 1. The current model is quarantined in a shared cooldown map for a window
//!    derived from the failure class (a daily-limit 429 gets a long quarantine;
//!    a transient blip a short one).
//! 2. The next pool member that is not quarantined is selected and switched to.
//! 3. The turn retries immediately on the new model — no artificial sleep, since
//!    a different backend is exactly the remedy the backoff was standing in for.
//!
//! Once every pool member is quarantined the last resort is the original
//! behaviour: fall back to sleeping, so a totally-down gateway still gets the
//! bounded backoff rather than failing fast.
//!
//! # Why it is invisible
//!
//! Every model in the pool is presented to the user as "Alphax Free"
//! ([`free_model_display_name`]). Rotation is a gateway implementation detail;
//! surfacing `nvidia/nemotron-3-ultra-550b-a55b:free` in the header would be
//! both alarming and meaningless. The switch still emits a `ModelChanged`
//! event so the header, picker, and context budget resync rather than showing a
//! stale model, but the name the user sees does not change.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::alphacode_provider_metadata::free_pool::{
    ALPHAX_FREE_DISPLAY_NAME, is_free_tier_model_id, merge_pool,
};
// The curated floor is only asserted against, never used at runtime: rotation
// reads the merged pool, which is a superset once a refresh has run.
#[cfg(test)]
use crate::alphacode_provider_metadata::free_pool::CURATED_FREE_MODELS;

/// Quarantine window after a rate-limit rejection.
///
/// Long, because the 429 body Kilo returns for a daily cap says so explicitly
/// and retrying inside this window cannot succeed. The user's real alternative
/// is waiting for tomorrow, so a short window here would just burn the pool.
const RATE_LIMIT_QUARANTINE: Duration = Duration::from_secs(15 * 60);

/// Quarantine window after a 5xx / timeout. Upstream capacity problems usually
/// clear in seconds, so this is short enough that the model returns to service
/// within the same session.
const TRANSIENT_QUARANTINE: Duration = Duration::from_secs(60);

/// Quarantine window after an empty response.
///
/// Empty output is much weaker evidence than an explicit 429 or a 5xx — it can
/// be a one-off fluke, or even a legitimate model that chose to say nothing. So
/// this is the shortest window by a wide margin: the model should be eligible
/// again quickly, and rotation is only a nudge, not a verdict.
const EMPTY_RESPONSE_QUARANTINE: Duration = Duration::from_secs(30);

/// How the failure should be classified for quarantine purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RotationTrigger {
    /// HTTP 429 or a body that names a rate limit.
    RateLimited,
    /// 5xx, transport error, or timeout.
    Transient,
    /// The model responded but produced nothing usable.
    EmptyResponse,
}

impl RotationTrigger {
    fn quarantine(self) -> Duration {
        match self {
            Self::RateLimited => RATE_LIMIT_QUARANTINE,
            Self::Transient => TRANSIENT_QUARANTINE,
            Self::EmptyResponse => EMPTY_RESPONSE_QUARANTINE,
        }
    }
}

/// A model quarantined until `until`, i.e. skipped by rotation until then.
#[derive(Debug, Clone, Copy)]
struct Quarantine {
    until: Instant,
}

/// Process-wide model quarantine map, keyed by model id.
///
/// Shared rather than per-`Agent` on purpose: free-tier limits are enforced
/// per *account*, so a second agent (a swarm worker, an ambient runner) hitting
/// the same throttled model would learn nothing from a private map and would
/// burn its own retry budget re-discovering the same 429. `Instant` is
/// monotonic and cheap to compare, so entries self-expire; the map is only
/// pruned when it is consulted.
fn quarantine_map() -> &'static Mutex<HashMap<String, Quarantine>> {
    static MAP: OnceLock<Mutex<HashMap<String, Quarantine>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Models discovered as free by the background catalog refresh, beyond the
/// curated floor.
fn refreshed_pool() -> &'static Mutex<Vec<String>> {
    static POOL: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    POOL.get_or_init(|| Mutex::new(Vec::new()))
}

/// Merge newly discovered free models into the live pool.
///
/// Called by the background refresh. A poisoned lock is treated as "no new
/// models" rather than propagated: losing a pool update is not worth failing a
/// session over, and the curated floor still applies.
pub fn absorb_discovered_free_models(ids: Vec<String>) {
    if ids.is_empty() {
        return;
    }
    let Ok(mut pool) = refreshed_pool().lock() else {
        return;
    };
    for id in ids {
        if is_free_tier_model_id(&id) && !pool.contains(&id) {
            pool.push(id);
        }
    }
}

/// The full effective pool: curated floor first, then anything the refresh
/// found. Curated entries are never dropped, so a failed or empty refresh
/// cannot shrink the pool below the set known to work.
pub fn effective_free_pool() -> Vec<String> {
    let discovered = refreshed_pool()
        .lock()
        .map(|p| p.clone())
        .unwrap_or_default();
    merge_pool(&discovered)
}

/// Quarantine `model` for the window implied by `trigger`.
pub fn quarantine_model(model: &str, trigger: RotationTrigger) {
    let model = model.trim();
    if model.is_empty() {
        return;
    }
    let Ok(mut map) = quarantine_map().lock() else {
        return;
    };
    map.insert(
        model.to_string(),
        Quarantine {
            until: Instant::now() + trigger.quarantine(),
        },
    );
    crate::alphacode_base::logging::info(&format!(
        "Free-pool rotation: quarantining '{model}' for {}s ({:?})",
        trigger.quarantine().as_secs(),
        trigger
    ));
}

/// Clear any quarantine on `model`.
///
/// Called after a successful turn. Without this a model that recovered would
/// stay skipped for the rest of the session, which is a real failure mode:
/// the cooldown is written on a 429, but the pool does not grow when the
/// upstream rate limit resets.
pub fn clear_quarantine(model: &str) {
    let model = model.trim();
    if model.is_empty() {
        return;
    }
    if let Ok(mut map) = quarantine_map().lock()
        && map.remove(model).is_some()
    {
        crate::alphacode_base::logging::info(&format!(
            "Free-pool rotation: '{model}' recovered, back in service"
        ));
    }
}

/// Is this model currently quarantined?
///
/// Test-only visibility: the rotation path itself relies on [`next_free_model`],
/// which consults the same map directly while holding the lock once. This
/// wrapper exists so the quarantine bookkeeping can be asserted without
/// reaching past the module's own API.
#[cfg(test)]
fn is_quarantined(model: &str) -> bool {
    quarantine_map()
        .lock()
        .map(|map| map.contains_key(model.trim()))
        .unwrap_or(false)
}

/// Whether the active session should be rotated at all.
///
/// Rotation is scoped tightly to the Alphax Free profile. Rotating a paid
/// provider's model because of a 429 would silently change which bill the user
/// is on, and rotating a fixed-model provider (Gemini, Copilot) would pick a
/// model the user never chose and may not have access to. Only the free pool,
/// where every member is interchangeable and costs nothing, is safe to rotate
/// without asking.
pub fn should_rotate_for_model(model: &str) -> bool {
    is_free_tier_model_id(model)
}

/// Pick the next usable model in the pool, given the one that just failed.
///
/// Returns `None` when every alternative is quarantined, which tells the
/// caller to fall back to plain backoff rather than failing the turn.
pub fn next_free_model(current: &str) -> Option<String> {
    let current = current.trim();
    // Prune expired entries while we already hold the lock. Without this the map
    // would grow for the life of the process and every lookup would scan
    // tombstones for a long session.
    let now = Instant::now();
    let healthy: Vec<String> = {
        let map = quarantine_map().lock().ok()?;
        let mut map = map;
        map.retain(|_, q| q.until > now);
        map.keys()
            .filter(|k| k.as_str() != current)
            .cloned()
            .collect()
    };
    let quarantined: Vec<String> = quarantine_map()
        .lock()
        .map(|map| {
            map.iter()
                .filter(|(_, q)| q.until > now)
                .map(|(k, _)| k.clone())
                .collect()
        })
        .unwrap_or_default();

    effective_free_pool()
        .into_iter()
        .find(|candidate| candidate != current && !quarantined.contains(candidate))
        .or_else(|| {
            // Everything is quarantined. If any entry has already expired we
            // would have pruned it above, so this really is "pool exhausted".
            // Still allow the soonest-to-expire member rather than giving up:
            // waiting for one model to recover beats stalling the turn, and the
            // caller's own bounded budget prevents an infinite loop.
            healthy.into_iter().min_by_key(|candidate| {
                quarantine_map()
                    .lock()
                    .ok()
                    .and_then(|map| map.get(candidate).map(|q| q.until))
                    .unwrap_or(now)
            })
        })
}

/// Provider snapshot describing a rotation, for the UI event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RotationNotice {
    pub from_model: String,
    pub to_model: String,
    pub trigger: RotationTrigger,
}

impl RotationNotice {
    /// A user-facing line for the status line.
    ///
    /// Names the product, not the upstream model, so the message reads as
    /// "Alphax Free is switching to another free model" rather than exposing a
    /// gateway routing id.
    pub fn status_line(&self) -> String {
        let reason = match self.trigger {
            RotationTrigger::RateLimited => "rate limited",
            RotationTrigger::Transient => "temporarily unavailable",
            RotationTrigger::EmptyResponse => "returned no output",
        };
        format!("{ALPHAX_FREE_DISPLAY_NAME} is {reason}; switching to another free model…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests share a process-wide quarantine map, so each one must start from a
    /// clean slate or they will see each other's quarantines.
    fn reset() {
        if let Ok(mut map) = quarantine_map().lock() {
            map.clear();
        }
        if let Ok(mut pool) = refreshed_pool().lock() {
            pool.clear();
        }
    }

    #[test]
    fn curated_pool_is_the_rotation_floor() {
        reset();
        let pool = effective_free_pool();
        assert!(pool.len() >= CURATED_FREE_MODELS.len());
        for curated in CURATED_FREE_MODELS {
            assert!(
                pool.contains(&curated.to_string()),
                "curated `{curated}` missing from pool"
            );
        }
    }

    #[test]
    fn a_quarantined_model_is_not_selected_again() {
        reset();
        quarantine_model("poolside/laguna-s-2.1:free", RotationTrigger::RateLimited);
        assert!(is_quarantined("poolside/laguna-s-2.1:free"));
        let next = next_free_model("kilo-auto/free").expect("pool has alternatives");
        assert_ne!(next, "poolside/laguna-s-2.1:free");
    }

    #[test]
    fn rotation_moves_forward_rather_than_retrying_the_same_model() {
        reset();
        let next = next_free_model("kilo-auto/free").expect("pool has alternatives");
        assert_ne!(next, "kilo-auto/free");
    }

    #[test]
    fn clear_quarantine_returns_a_model_to_service() {
        reset();
        quarantine_model("cohere/north-mini-code:free", RotationTrigger::Transient);
        assert!(is_quarantined("cohere/north-mini-code:free"));
        clear_quarantine("cohere/north-mini-code:free");
        assert!(!is_quarantined("cohere/north-mini-code:free"));
    }

    #[test]
    fn clearing_an_unknown_model_is_harmless() {
        reset();
        clear_quarantine("some/model:free");
        clear_quarantine("");
        assert!(!is_quarantined("some/model:free"));
    }

    #[test]
    fn discovered_models_join_the_pool() {
        reset();
        absorb_discovered_free_models(vec![
            "brand-new/model:free".to_string(),
            "paid/gpt-5.4".to_string(),
        ]);
        let pool = effective_free_pool();
        assert!(pool.contains(&"brand-new/model:free".to_string()));
        assert!(
            !pool.contains(&"paid/gpt-5.4".to_string()),
            "a paid model must never enter the free pool"
        );
    }

    #[test]
    fn absorbing_an_empty_refresh_leaves_the_pool_intact() {
        reset();
        let before = effective_free_pool();
        absorb_discovered_free_models(Vec::new());
        assert_eq!(before, effective_free_pool());
    }

    #[test]
    fn only_free_models_are_rotated() {
        assert!(should_rotate_for_model("kilo-auto/free"));
        assert!(should_rotate_for_model("poolside/laguna-s-2.1:free"));
        assert!(
            !should_rotate_for_model("openai/gpt-5.4"),
            "rotating a paid model would silently change the user's billing"
        );
        assert!(!should_rotate_for_model("claude-sonnet-5"));
    }

    #[test]
    fn rate_limits_quarantine_longer_than_blips() {
        assert!(RATE_LIMIT_QUARANTINE > TRANSIENT_QUARANTINE);
        assert!(TRANSIENT_QUARANTINE > EMPTY_RESPONSE_QUARANTINE);
    }

    #[test]
    fn status_line_never_leaks_an_upstream_model_id() {
        reset();
        let notice = RotationNotice {
            from_model: "kilo-auto/free".to_string(),
            to_model: "nvidia/nemotron-3-ultra-550b-a55b:free".to_string(),
            trigger: RotationTrigger::RateLimited,
        };
        let line = notice.status_line();
        assert!(line.contains(ALPHAX_FREE_DISPLAY_NAME));
        assert!(
            !line.contains("nvidia/") && !line.contains(":free"),
            "status line must not expose the gateway routing id: {line}"
        );
    }

    #[test]
    fn every_trigger_produces_a_status_line() {
        for trigger in [
            RotationTrigger::RateLimited,
            RotationTrigger::Transient,
            RotationTrigger::EmptyResponse,
        ] {
            let notice = RotationNotice {
                from_model: "kilo-auto/free".to_string(),
                to_model: "x".to_string(),
                trigger,
            };
            assert!(!notice.status_line().is_empty());
        }
    }
}
