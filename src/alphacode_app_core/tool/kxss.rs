use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct KxssTool;

#[derive(Deserialize)]
struct KxssInput {
    /// Input file or URL
    input: String,
    /// Pattern to search for
    #[serde(default)]
    pattern: Option<String>,
    /// Case insensitive
    #[serde(default)]
    case_insensitive: bool,
    /// Invert match
    #[serde(default)]
    invert: bool,
    /// Max matches
    #[serde(default)]
    max_matches: Option<usize>,
    /// Output file
    #[serde(default)]
    output: Option<String>,
}

#[async_trait]
impl Tool for KxssTool {
    fn name(&self) -> &str {
        "kxss"
    }

    fn description(&self) -> &str {
        "XSS reflection finder. Use to find reflected parameters in URLs that may be vulnerable to XSS."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["input"],
            "properties": {
                "intent": super::intent_schema_property(),
                "input": {
                    "type": "string",
                    "description": "Input file or URL."
                },
                "pattern": {
                    "type": "string",
                    "description": "Pattern to search for."
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive matching. Default: false."
                },
                "invert": {
                    "type": "boolean",
                    "description": "Invert match. Default: false."
                },
                "max_matches": {
                    "type": "integer",
                    "description": "Maximum matches to return."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: KxssInput = normalize_kxss_input(&input)?;
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "kxss",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("kxss"))
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
            return Err(anyhow::anyhow!("kxss exited with error: {detail}"));
        }

        let (lines, total, truncated) = super::recon_common::parse_lines(&output.stdout);

        let mut result = format!("kxss found {} matches:\n\n", total);
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
            .with_title(format!("kxss: {total} matches"))
            .with_metadata(json!(metadata)))
    }
}

impl KxssTool {
    pub fn new() -> Self {
        Self
    }
}

fn normalize_kxss_input(input: &Value) -> Result<KxssInput> {
    if let Ok(params) = serde_json::from_value::<KxssInput>(input.clone()) {
        return Ok(params);
    }
    let obj = input.as_object().ok_or_else(|| {
        anyhow::anyhow!("kxss expects a JSON object with `input`, e.g. {{\"input\": \"urls.txt\"}}")
    })?;
    let input_val = ["input", "file", "i", "url"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing field `input`. Provide the input as `input`."))?;
    Ok(KxssInput {
        input: input_val.to_string(),
        pattern: obj
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(String::from),
        case_insensitive: obj
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        invert: obj.get("invert").and_then(|v| v.as_bool()).unwrap_or(false),
        max_matches: obj
            .get("max_matches")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        output: obj.get("output").and_then(|v| v.as_str()).map(String::from),
    })
}

fn build_args(params: &KxssInput) -> Result<Vec<String>> {
    let mut args = Vec::new();

    args.push(params.input.clone());

    if let Some(ref pattern) = params.pattern {
        args.push("-p".to_string());
        args.push(pattern.clone());
    }
    if params.case_insensitive {
        args.push("-i".to_string());
    }
    if params.invert {
        args.push("-v".to_string());
    }
    if let Some(max) = params.max_matches {
        args.push("-m".to_string());
        args.push(max.to_string());
    }
    if let Some(ref output) = params.output {
        args.push("-o".to_string());
        args.push(output.clone());
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_accepts_aliases() {
        for payload in [
            serde_json::json!({"input": "urls.txt"}),
            serde_json::json!({"file": "urls.txt"}),
        ] {
            let params = normalize_kxss_input(&payload).expect("normalize");
            assert_eq!(params.input, "urls.txt");
        }
    }
}
