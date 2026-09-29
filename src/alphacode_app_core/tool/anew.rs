use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct AnewTool;

#[derive(Deserialize)]
struct AnewInput {
    /// Input file
    input: String,
    /// Output file
    #[serde(default)]
    output: Option<String>,
    /// Append mode
    #[serde(default)]
    append: bool,
    /// Unique only
    #[serde(default)]
    unique: bool,
    /// Sort
    #[serde(default)]
    sort: bool,
}

#[async_trait]
impl Tool for AnewTool {
    fn name(&self) -> &str {
        "anew"
    }

    fn description(&self) -> &str {
        "Append-only deduplication tool. Use to merge and deduplicate URL/host lists from multiple sources."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["input"],
            "properties": {
                "intent": super::intent_schema_property(),
                "input": {
                    "type": "string",
                    "description": "Input file with entries (one per line)."
                },
                "output": {
                    "type": "string",
                    "description": "Output file path."
                },
                "append": {
                    "type": "boolean",
                    "description": "Append mode. Default: false."
                },
                "unique": {
                    "type": "boolean",
                    "description": "Only unique entries. Default: false."
                },
                "sort": {
                    "type": "boolean",
                    "description": "Sort output. Default: false."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: AnewInput = normalize_anew_input(&input)?;
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "anew",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("anew"))
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
            return Err(anyhow::anyhow!("anew exited with error: {detail}"));
        }

        let (lines, total, truncated) = super::recon_common::parse_lines(&output.stdout);

        let mut result = format!("anew found {} unique entries:\n\n", total);
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
            .with_title(format!("anew: {total} unique entries"))
            .with_metadata(json!(metadata)))
    }
}

impl AnewTool {
    pub fn new() -> Self {
        Self
    }
}

fn normalize_anew_input(input: &Value) -> Result<AnewInput> {
    if let Ok(params) = serde_json::from_value::<AnewInput>(input.clone()) {
        return Ok(params);
    }
    let obj = input.as_object().ok_or_else(|| {
        anyhow::anyhow!("anew expects a JSON object with `input`, e.g. {{\"input\": \"urls.txt\"}}")
    })?;
    let input_val = ["input", "file", "i"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing field `input`. Provide the input as `input`."))?;
    Ok(AnewInput {
        input: input_val.to_string(),
        output: obj.get("output").and_then(|v| v.as_str()).map(String::from),
        append: obj.get("append").and_then(|v| v.as_bool()).unwrap_or(false),
        unique: obj.get("unique").and_then(|v| v.as_bool()).unwrap_or(false),
        sort: obj.get("sort").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn build_args(params: &AnewInput) -> Result<Vec<String>> {
    let mut args = Vec::new();

    args.push(params.input.clone());

    if let Some(ref output) = params.output {
        args.push(output.clone());
    }
    if params.append {
        args.push("-a".to_string());
    }
    if params.unique {
        args.push("-u".to_string());
    }
    if params.sort {
        args.push("-s".to_string());
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
            let params = normalize_anew_input(&payload).expect("normalize");
            assert_eq!(params.input, "urls.txt");
        }
    }
}
