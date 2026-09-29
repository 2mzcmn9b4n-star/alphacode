use serde::{Deserialize, Serialize};

use super::context::SecurityContext;
use super::finding::{Confidence, Finding, Severity};

/// Best-effort extraction of the hostname from a finding target, which may be
/// a bare host (`api.example.com`), a full URL (`https://api.example.com/v1/x`),
/// or a `host:port` pair. Returns `None` for anything that has no host part
/// (empty string, `user@` forms with no host, `/`-only paths).
fn host_of(target: &str) -> Option<String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Strip a scheme if present so `https://host/path` and `host/path` agree.
    let after_scheme = match trimmed.find("://") {
        Some(idx) => &trimmed[idx + 3..],
        None => trimmed,
    };
    // Take the authority component: up to the first `/`, `?` or `#`.
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    // Drop any `userinfo@` prefix, then the port.
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = match host.rfind(':') {
        // Only strip when the trailing part is a port (digits), not part of a
        // bare IPv6 literal.
        Some(idx) if host[idx + 1..].chars().all(|c| c.is_ascii_digit()) => &host[..idx],
        _ => host,
    };
    if host.is_empty() {
        None
    } else {
        Some(host.to_ascii_lowercase())
    }
}

/// Gate results for the validation pipeline.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GateResult {
    pub gate: ValidationGate,
    pub passed: bool,
    pub reasoning: String,
    pub recommendations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationGate {
    Reproducibility,
    SecurityRelevance,
    BoundaryViolation,
    ImpactDemonstration,
    ControlComparison,
    InformationalCheck,
    Reportability,
}

impl ValidationGate {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Reproducibility => "reproducibility",
            Self::SecurityRelevance => "security_relevance",
            Self::BoundaryViolation => "boundary_violation",
            Self::ImpactDemonstration => "impact_demonstration",
            Self::ControlComparison => "control_comparison",
            Self::InformationalCheck => "informational_check",
            Self::Reportability => "reportability",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Reproducibility => "Can the agent reproduce it independently?",
            Self::SecurityRelevance => "Is this behavior actually security-relevant?",
            Self::BoundaryViolation => "Is there a security boundary violation?",
            Self::ImpactDemonstration => "Can the impact be demonstrated?",
            Self::ControlComparison => "Is there a control/baseline comparison?",
            Self::InformationalCheck => "Could this simply be informational?",
            Self::Reportability => "Is it actually reportable given scope and rules?",
        }
    }
}

/// Full validation result for a candidate finding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub finding_id: String,
    pub gates: Vec<GateResult>,
    pub overall_passed: bool,
    pub final_confidence: Confidence,
    pub final_severity: Severity,
    pub validator_notes: String,
    pub adversarial_review: Option<AdversarialReview>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdversarialReview {
    pub reviewer: String,
    pub attempted_disproof: Vec<String>,
    pub disproof_successful: bool,
    pub alternative_explanations: Vec<String>,
    pub verdict: String,
    pub confidence_adjustment: Option<f64>,
}

/// The validation engine runs candidate findings through 7 mandatory gates.
pub struct ValidationEngine;

impl ValidationEngine {
    /// Validate a candidate finding through all 7 gates.
    pub fn validate(finding: &Finding, context: &SecurityContext) -> ValidationResult {
        let gates = vec![
            Self::gate_reproducibility(finding),
            Self::gate_security_relevance(finding),
            Self::gate_boundary_violation(finding),
            Self::gate_impact_demonstration(finding),
            Self::gate_control_comparison(finding, context),
            Self::gate_informational_check(finding),
            Self::gate_reportability(finding, context),
        ];

        let overall_passed = gates.iter().all(|g| g.passed);
        let passed_count = gates.iter().filter(|g| g.passed).count();
        let final_confidence = if overall_passed {
            Confidence::Certain
        } else if passed_count >= 5 {
            Confidence::High
        } else if passed_count >= 3 {
            Confidence::Medium
        } else {
            Confidence::Low
        };

        ValidationResult {
            finding_id: finding.id.clone(),
            gates,
            overall_passed,
            final_confidence,
            final_severity: finding.severity.clone(),
            validator_notes: String::new(),
            adversarial_review: None,
        }
    }

    fn gate_reproducibility(finding: &Finding) -> GateResult {
        let has_repro = finding.reproduction.is_some();
        GateResult {
            gate: ValidationGate::Reproducibility,
            passed: has_repro,
            reasoning: if has_repro {
                "Reproduction steps provided".to_string()
            } else {
                "No reproduction steps — cannot verify independently".to_string()
            },
            recommendations: if has_repro {
                vec![]
            } else {
                vec!["Add detailed reproduction steps".to_string()]
            },
        }
    }

    fn gate_security_relevance(finding: &Finding) -> GateResult {
        // A *named* custom class is a real, specific vulnerability class and
        // must pass. This matters a lot in practice: web3 findings arrive
        // exclusively as `Custom(..)` because `VulnerabilityClass` has no
        // native EVM variants (see `skill_router`), so rejecting every
        // `Custom` made the whole web3 pipeline unable to ever reach
        // `overall_passed` / `Certain` confidence. Only an empty or
        // placeholder name means "not actually classified".
        let is_unclassified = match &finding.vuln_class {
            super::finding::VulnerabilityClass::Custom(name) => name.trim().is_empty(),
            _ => false,
        };
        let is_tagged_informational = finding.tags.iter().any(|t| t == "informational");
        let is_relevant = !is_unclassified && !is_tagged_informational;
        GateResult {
            gate: ValidationGate::SecurityRelevance,
            passed: is_relevant,
            reasoning: if is_relevant {
                "Vulnerability class is security-relevant".to_string()
            } else if is_tagged_informational {
                "Tagged informational rather than a security issue".to_string()
            } else {
                "Vulnerability class is unclassified; name the specific class".to_string()
            },
            recommendations: vec![],
        }
    }

    fn gate_boundary_violation(finding: &Finding) -> GateResult {
        let has_boundary = !finding.boundary_violations.is_empty();
        GateResult {
            gate: ValidationGate::BoundaryViolation,
            passed: has_boundary,
            reasoning: if has_boundary {
                format!(
                    "{} boundary violation(s) identified",
                    finding.boundary_violations.len()
                )
            } else {
                "No security boundary violation identified".to_string()
            },
            recommendations: if has_boundary {
                vec![]
            } else {
                vec![
                    "Identify which security boundary is violated".to_string(),
                    "Document authentication/authorization context".to_string(),
                ]
            },
        }
    }

    fn gate_impact_demonstration(finding: &Finding) -> GateResult {
        let has_impact = finding.impact.is_some();
        GateResult {
            gate: ValidationGate::ImpactDemonstration,
            passed: has_impact,
            reasoning: if has_impact {
                "Impact has been demonstrated".to_string()
            } else {
                "No concrete impact demonstrated".to_string()
            },
            recommendations: if has_impact {
                vec![]
            } else {
                vec!["Demonstrate concrete security impact".to_string()]
            },
        }
    }

    fn gate_control_comparison(finding: &Finding, _context: &SecurityContext) -> GateResult {
        let has_baseline = finding
            .notes
            .iter()
            .any(|n| n.contains("expected") || n.contains("baseline") || n.contains("control"));
        GateResult {
            gate: ValidationGate::ControlComparison,
            passed: has_baseline,
            reasoning: if has_baseline {
                "Control/baseline comparison present".to_string()
            } else {
                "No comparison with expected/baseline behavior".to_string()
            },
            recommendations: if has_baseline {
                vec![]
            } else {
                vec!["Compare attacker behavior vs expected behavior".to_string()]
            },
        }
    }

    fn gate_informational_check(finding: &Finding) -> GateResult {
        let is_info_only = finding.severity == super::finding::Severity::Info
            && finding
                .impact
                .as_ref()
                .map(|i| {
                    i.confidentiality.is_none() && i.integrity.is_none() && i.availability.is_none()
                })
                .unwrap_or(true);
        GateResult {
            gate: ValidationGate::InformationalCheck,
            passed: !is_info_only,
            reasoning: if is_info_only {
                "Finding appears informational only — no meaningful security impact".to_string()
            } else {
                "Finding has security impact beyond informational".to_string()
            },
            recommendations: if is_info_only {
                vec!["Consider downgrading to informational or removing".to_string()]
            } else {
                vec![]
            },
        }
    }

    fn gate_reportability(finding: &Finding, context: &SecurityContext) -> GateResult {
        let excluded = context
            .scope
            .is_vuln_class_excluded(&finding.vuln_class.as_str());
        // The live scope verdict is part of "is it actually reportable". A
        // finding on a host the program explicitly excluded must not be able
        // to pass every gate and land at `Certain` confidence. Check both the
        // declared target and the specific endpoint's host.
        let out_of_scope_targets = [Some(finding.target.as_str()), finding.endpoint.as_deref()]
            .into_iter()
            .flatten()
            .filter(|t| !t.is_empty())
            .filter_map(host_of)
            .any(|host| context.scope.is_out_of_scope(&host));
        let is_reportable = !excluded
            && !out_of_scope_targets
            && finding.severity != super::finding::Severity::Info;
        GateResult {
            gate: ValidationGate::Reportability,
            passed: is_reportable,
            reasoning: if excluded {
                "Vulnerability class is excluded by program rules".to_string()
            } else if out_of_scope_targets {
                format!("Target '{}' is marked out of scope", finding.target)
            } else if !is_reportable {
                "Info-only findings are not reportable".to_string()
            } else {
                "Finding is reportable given scope and rules".to_string()
            },
            recommendations: if out_of_scope_targets {
                vec!["Confirm the target is in the program scope before reporting".to_string()]
            } else {
                vec![]
            },
        }
    }
}

/// Multi-agent quality control: independent validator + adversarial reviewer.
pub struct QualityControl;

impl QualityControl {
    /// Run adversarial review to attempt to disprove a finding.
    pub fn adversarial_review(
        finding: &Finding,
        validation: &ValidationResult,
        reviewer: &str,
    ) -> AdversarialReview {
        let mut attempted_disproof = Vec::new();
        let mut alternative_explanations = Vec::new();

        // Check for common false positive patterns
        if finding.auth_context == "unauthenticated"
            && finding.vuln_class == super::finding::VulnerabilityClass::IdorBola
        {
            attempted_disproof.push(
                "IDOR requires authentication context — unauthenticated access may be intended"
                    .to_string(),
            );
            alternative_explanations
                .push("Endpoint may be publicly accessible by design".to_string());
        }

        if finding.severity == super::finding::Severity::Critical
            && finding.confidence == super::finding::Confidence::Low
        {
            attempted_disproof
                .push("High severity with low confidence — likely overstated".to_string());
            alternative_explanations.push("Behavior may be intended functionality".to_string());
        }

        // Check boundary violations
        if finding.boundary_violations.is_empty() {
            attempted_disproof.push("No boundary violation documented".to_string());
        }

        // Check reproduction
        if finding.reproduction.is_none() {
            attempted_disproof.push("Cannot reproduce independently".to_string());
        }

        let disproof_successful = !validation.overall_passed
            || attempted_disproof.len() > 2
            || alternative_explanations.len() > 1;

        let verdict = if disproof_successful {
            "Finding could not survive adversarial review — likely false positive".to_string()
        } else {
            "Finding withstands adversarial scrutiny".to_string()
        };

        AdversarialReview {
            reviewer: reviewer.to_string(),
            attempted_disproof,
            disproof_successful,
            alternative_explanations,
            verdict,
            confidence_adjustment: if disproof_successful {
                Some(-0.25)
            } else {
                Some(0.1)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::finding::VulnerabilityClass;
    use super::super::scope::{EngagementMode, LiveScope};
    use super::*;

    fn test_finding() -> Finding {
        Finding::new(
            "f1".into(),
            "Test XSS".into(),
            VulnerabilityClass::Xss,
            "target".into(),
        )
    }

    fn test_context() -> SecurityContext {
        let scope = LiveScope::new(EngagementMode::BugBounty, "test-target".into());
        SecurityContext::new("eng-1".into(), "Test".into(), scope)
    }

    #[test]
    fn validation_fails_by_default_for_bare_finding() {
        let finding = test_finding();
        let context = test_context();
        let result = ValidationEngine::validate(&finding, &context);
        assert!(!result.overall_passed);
        assert_eq!(result.gates.len(), 7);
        // A fresh Xss finding passes security-relevance but little else.
        let passed = result.gates.iter().filter(|g| g.passed).count();
        assert!(
            passed <= 2,
            "bare finding should pass at most 2 gates, got {passed}"
        );
    }

    #[test]
    fn reproducibility_gate() {
        let mut finding = test_finding();
        finding.reproduction = Some(super::super::finding::ReproductionSteps {
            preconditions: vec![],
            steps: vec!["Step 1".into()],
            expected_behavior: "Safe output".into(),
            actual_behavior: "XSS executed".into(),
            success_indicator: "alert(1)".into(),
        });
        let context = test_context();
        let result = ValidationEngine::validate(&finding, &context);
        let repro_gate = result
            .gates
            .iter()
            .find(|g| g.gate == ValidationGate::Reproducibility)
            .unwrap();
        assert!(repro_gate.passed);
    }

    #[test]
    fn adversarial_review_disproves_weak_finding() {
        let finding = test_finding();
        let context = test_context();
        let validation = ValidationEngine::validate(&finding, &context);
        let review = QualityControl::adversarial_review(&finding, &validation, "reviewer-1");
        assert!(review.disproof_successful);
    }
}
