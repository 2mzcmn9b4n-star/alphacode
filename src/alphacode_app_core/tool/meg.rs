use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct MegTool;

#[derive(Deserialize)]
struct MegInput {
    /// Input: URL or file path
    input: String,
    /// Output path
    #[serde(default)]
    output: Option<String>,
    /// Depth
    #[serde(default)]
    depth: Option<usize>,
    /// Concurrent requests
    #[serde(default)]
    concurrency: Option<usize>,
    /// Timeout
    #[serde(default)]
    timeout: Option<u64>,
    /// Follow redirects
    #[serde(default)]
    follow_redirects: bool,
    /// Silent
    #[serde(default)]
    silent: bool,
}

#[async_trait]
impl Tool for MegTool {
    fn name(&self) -> &str {
        "meg"
    }

    fn description(&self) -> &str {
        "URL extraction at scale. Fetches paths from a list of URLs and extracts all links. Use to build large URL lists for further analysis."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["input"],
            "properties": {
                "intent": super::intent_schema_property(),
                "input": {
                    "type": "string",
                    "description": "URL or file path with URLs (one per line)."
                },
                "output": {
                    "type": "string",
                    "description": "Output file path."
                },
                "depth": {
                    "type": "integer",
                    "description": "Crawl depth. Default: 2."
                },
                "concurrency": {
                    "type": "integer",
                    "description": "Concurrent requests. Default: 20."
                },
                "follow_redirects": {
                    "type": "boolean",
                    "description": "Follow redirects. Default: false."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: MegInput = normalize_meg_input(&input)?;
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "meg",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("meg"))
            } else {
                anyhow::anyhow!("{e}")
            }
        })?;

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if !output.status.success() {
            let detail = if stderr.is_empty() {
                if stdout.is_empty() {
                    "no output (binary exited non-zero with empty stderr)".to_string()
                } else {
                    crate::alphacode_core::util::truncate_str(&stdout, 500).to_string()
                }
            } else {
                crate::alphacode_core::util::truncate_str(&stderr, 500).to_string()
            };
            return Err(anyhow::anyhow!("meg exited with error: {detail}"));
        }

        let (lines, total, truncated) = super::recon_common::parse_lines(&output.stdout);

        let mut result = format!("meg found {} URLs:\n\n", total);
        for line in &lines {
            result.push_str(line);
            result.push('\n');
        }
        result.push_str(&super::recon_common::truncation_notice(lines.len(), total));

        let mut metadata = HashMap::new();
        metadata.insert("input".to_string(), json!(params.input));
        metadata.insert("count".to_string(), json!(lines.len()));
        metadata.insert("total_found".to_string(), json!(total));
        metadata.insert("truncated".to_string(), json!(truncated));

        Ok(ToolOutput::new(result)
            .with_title(format!("meg: {total} URLs"))
            .with_metadata(json!(metadata)))
    }
}

impl MegTool {
    pub fn new() -> Self {
        Self
    }
}

fn normalize_meg_input(input: &Value) -> Result<MegInput> {
    if let Ok(params) = serde_json::from_value::<MegInput>(input.clone()) {
        return Ok(params);
    }
    let obj = input.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "meg expects a JSON object with `input`, e.g. {{\"input\": \"https://example.com\"}}"
        )
    })?;
    let input_val = ["input", "url", "target", "i"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing field `input`. Provide the input as `input`."))?;
    Ok(MegInput {
        input: input_val.to_string(),
        output: obj.get("output").and_then(|v| v.as_str()).map(String::from),
        depth: obj
            .get("depth")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        concurrency: obj
            .get("concurrency")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        timeout: obj.get("timeout").and_then(|v| v.as_u64()),
        follow_redirects: obj
            .get("follow_redirects")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        silent: obj.get("silent").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn build_args(params: &MegInput) -> Result<Vec<String>> {
    let mut args = Vec::new();

    args.push(params.input.clone());

    if let Some(ref output) = params.output {
        args.push(output.clone());
    }

    if let Some(depth) = params.depth {
        args.push("--depth".to_string());
        args.push(depth.to_string());
    }
    if let Some(concurrency) = params.concurrency {
        args.push("--concurrency".to_string());
        args.push(concurrency.to_string());
    }
    if let Some(timeout) = params.timeout {
        args.push("--timeout".to_string());
        args.push(timeout.to_string());
    }
    if params.follow_redirects {
        args.push("--follow-redirects".to_string());
    }
    if params.silent {
        args.push("--silent".to_string());
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_accepts_aliases() {
        for payload in [
            serde_json::json!({"input": "https://example.com"}),
            serde_json::json!({"url": "https://example.com"}),
        ] {
            let params = normalize_meg_input(&payload).expect("normalize");
            assert_eq!(params.input, "https://example.com");
        }
    }
}
