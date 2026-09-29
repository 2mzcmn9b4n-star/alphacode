use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const DEFAULT_LIMIT: usize = 1000;

pub struct WaybackurlsTool;

#[derive(Deserialize)]
struct WaybackurlsInput {
    domain: String,
    /// Applied locally after the run. waybackurls itself has no `-limit` flag
    /// (only `-dates`, `-no-subs`, `-get-versions`), so the cap is ours to
    /// enforce; the true total is always reported alongside the cap.
    #[serde(default)]
    limit: Option<usize>,
    /// `waybackurls -no-subs`: drop subdomains of the target domain.
    #[serde(default, alias = "no_color")]
    no_subs: bool,
    /// `waybackurls -dates`: prefix each URL with its fetch date.
    #[serde(default)]
    dates: bool,
}

#[async_trait]
impl Tool for WaybackurlsTool {
    fn name(&self) -> &str {
        "waybackurls"
    }

    fn description(&self) -> &str {
        "Fetch all known URLs for a domain from the Wayback Machine. Great for discovering historical endpoints, hidden parameters, and old versions of files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["domain"],
            "properties": {
                "intent": super::intent_schema_property(),
                "domain": {
                    "type": "string",
                    "description": "Target domain to fetch URLs for."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of URLs to return. Applied locally after the run; the total found is always reported. Default: 1000."
                },
                "no_subs": {
                    "type": "boolean",
                    "description": "Exclude subdomains of the target domain (waybackurls -no-subs). Default: false."
                },
                "dates": {
                    "type": "boolean",
                    "description": "Prefix each URL with its archive fetch date (waybackurls -dates). Default: false."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: WaybackurlsInput = serde_json::from_value(input)?;
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "waybackurls",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("waybackurls"))
            } else {
                anyhow::anyhow!("{e}")
            }
        })?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "{}",
                super::recon_common::describe_failure("waybackurls", &output)
            ));
        }

        let (all_urls, total, _) = super::recon_common::parse_lines(&output.stdout);

        // `limit` is enforced *here*, not by the tool. waybackurls has no such
        // flag (it registers only -dates, -no-subs and -get-versions), so the
        // cap has to be local — and the true total is reported so a truncated
        // result is never mistaken for a complete one.
        let requested_limit = params.limit.unwrap_or(DEFAULT_LIMIT) as usize;
        let truncated_by_limit = all_urls.len() > requested_limit;
        let urls: Vec<String> = all_urls.into_iter().take(requested_limit).collect();

        let mut result = format!(
            "waybackurls found {} URLs for {}:\n\n",
            total, params.domain
        );

        for url in &urls {
            result.push_str(url);
            result.push('\n');
        }
        if truncated_by_limit {
            result.push_str(&format!(
                "\n[showing the first {} of {total} URLs — the limit was requested; re-run with a higher `limit` for more]\n",
                urls.len()
            ));
        }

        let mut metadata = HashMap::new();
        metadata.insert("domain".to_string(), json!(params.domain));
        metadata.insert("count".to_string(), json!(urls.len()));
        metadata.insert("total_found".to_string(), json!(total));
        metadata.insert("limit".to_string(), json!(requested_limit));
        metadata.insert("limited".to_string(), json!(truncated_by_limit));
        metadata.insert("no_subs".to_string(), json!(params.no_subs));
        metadata.insert("dates".to_string(), json!(params.dates));

        Ok(ToolOutput::new(result)
            .with_title(format!("waybackurls: {total} URLs found"))
            .with_metadata(json!(metadata)))
    }
}

impl WaybackurlsTool {
    pub fn new() -> Self {
        Self
    }
}

/// Build argv for waybackurls.
///
/// waybackurls uses the Go stdlib `flag` package, which registers exactly
/// three flags: `-dates`, `-no-subs`, `-get-versions`. The previous code
/// emitted `-limit` and `-no-color`, neither of which exists, so *every*
/// invocation failed with "flag provided but not defined" and exit code 2.
/// `limit` is therefore applied locally after the fact.
fn build_args(params: &WaybackurlsInput) -> Result<Vec<String>> {
    let domain = super::recon_common::validate_hostname(&params.domain)?;

    let mut args = Vec::new();
    if params.dates {
        args.push("-dates".to_string());
    }
    if params.no_subs {
        args.push("-no-subs".to_string());
    }
    // `--` terminates flag parsing so a positional domain can never be read as
    // a flag, even after the validation above.
    args.push("--".to_string());
    args.push(domain);

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(domain: &str) -> WaybackurlsInput {
        WaybackurlsInput {
            domain: domain.to_string(),
            limit: None,
            no_subs: false,
            dates: false,
        }
    }

    /// Regression: the old builder emitted `-limit` and `-no-color`, neither of
    /// which waybackurls registers. Go's `flag` package exits 2 on an unknown
    /// flag, so *every* call failed before doing any work.
    #[test]
    fn only_real_waybackurls_flags_are_emitted() {
        let args = build_args(&input("example.com")).expect("args");
        assert!(
            !args.iter().any(|a| a == "-limit"),
            "-limit is not a waybackurls flag: {args:?}"
        );
        assert!(
            !args.iter().any(|a| a == "-no-color"),
            "-no-color is not a waybackurls flag: {args:?}"
        );
        assert!(args.contains(&"example.com".to_string()));
        // The domain must follow `--` so it can never be read as a flag.
        let sep = args.iter().position(|a| a == "--").expect("-- present");
        assert_eq!(args[sep + 1], "example.com");
    }

    #[test]
    fn real_flags_are_forwarded() {
        let mut i = input("example.com");
        i.dates = true;
        i.no_subs = true;
        let args = build_args(&i).expect("args");
        assert!(args.contains(&"-dates".to_string()));
        assert!(args.contains(&"-no-subs".to_string()));
    }

    #[test]
    fn flag_like_domain_is_rejected() {
        // `-dates` and `-get-versions` are real waybackurls flags, so a domain
        // of "-dates" would silently switch the tool into another mode.
        assert!(build_args(&input("-dates")).is_err());
        assert!(build_args(&input("-get-versions")).is_err());
        assert!(build_args(&input("example.com -o /tmp")).is_err());
    }

    /// The cap is enforced locally because the tool cannot do it, and the
    /// total is always reported so a capped list is not read as complete.
    #[test]
    fn limit_is_a_local_cap_not_a_flag() {
        let mut i = input("example.com");
        i.limit = Some(5);
        let args = build_args(&i).expect("args");
        assert!(
            !args.iter().any(|a| a == "5"),
            "limit leaked into argv: {args:?}"
        );
        assert_eq!(i.limit, Some(5), "limit must survive for local capping");
    }
}
