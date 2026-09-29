use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct QsreplaceTool;

#[derive(Deserialize)]
struct QsreplaceInput {
    /// Input file or URL
    input: String,
    /// Replacement value
    #[serde(default)]
    value: Option<String>,
    /// Parameter to replace
    #[serde(default)]
    param: Option<String>,
    /// All parameters
    #[serde(default)]
    all_params: bool,
    /// Output file
    #[serde(default)]
    output: Option<String>,
    /// JSON output
    #[serde(default)]
    json: bool,
    /// Silent
    #[serde(default)]
    silent: bool,
    /// Unique
    #[serde(default)]
    unique: bool,
    /// Sort
    #[serde(default)]
    sort: bool,
}

#[async_trait]
impl Tool for QsreplaceTool {
    fn name(&self) -> &str {
        "qsreplace"
    }

    fn description(&self) -> &str {
        "Query string parameter replacer. Use to replace parameter values in URL lists for testing."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["input"],
            "properties": {
                "intent": super::intent_schema_property(),
                "input": {
                    "type": "string",
                    "description": "Input file with URLs (one per line)."
                },
                "value": {
                    "type": "string",
                    "description": "Replacement value."
                },
                "param": {
                    "type": "string",
                    "description": "Parameter to replace."
                },
                "all_params": {
                    "type": "boolean",
                    "description": "Replace all parameters. Default: false."
                },
                "output": {
                    "type": "string",
                    "description": "Output file path."
                },
                "unique": {
                    "type": "boolean",
                    "description": "Only unique results. Default: false."
                },
                "sort": {
                    "type": "boolean",
                    "description": "Sort output. Default: false."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: QsreplaceInput = normalize_qsreplace_input(&input)?;
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "qsreplace",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("qsreplace"))
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
            return Err(anyhow::anyhow!("qsreplace exited with error: {detail}"));
        }

        let (lines, total, truncated) = super::recon_common::parse_lines(&output.stdout);

        let mut result = format!("qsreplace found {} results:\n\n", total);
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
            .with_title(format!("qsreplace: {total} results"))
            .with_metadata(json!(metadata)))
    }
}

impl QsreplaceTool {
    pub fn new() -> Self {
        Self
    }
}

fn normalize_qsreplace_input(input: &Value) -> Result<QsreplaceInput> {
    if let Ok(params) = serde_json::from_value::<QsreplaceInput>(input.clone()) {
        return Ok(params);
    }
    let obj = input.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "qsreplace expects a JSON object with `input`, e.g. {{\"input\": \"urls.txt\"}}"
        )
    })?;
    let input_val = ["input", "file", "i", "urls"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing field `input`. Provide the input as `input`."))?;
    Ok(QsreplaceInput {
        input: input_val.to_string(),
        value: obj.get("value").and_then(|v| v.as_str()).map(String::from),
        param: obj.get("param").and_then(|v| v.as_str()).map(String::from),
        all_params: obj
            .get("all_params")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        output: obj.get("output").and_then(|v| v.as_str()).map(String::from),
        json: obj.get("json").and_then(|v| v.as_bool()).unwrap_or(false),
        silent: obj.get("silent").and_then(|v| v.as_bool()).unwrap_or(false),
        unique: obj.get("unique").and_then(|v| v.as_bool()).unwrap_or(false),
        sort: obj.get("sort").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn build_args(params: &QsreplaceInput) -> Result<Vec<String>> {
    let mut args = Vec::new();

    args.push(params.input.clone());

    if let Some(ref value) = params.value {
        args.push("-v".to_string());
        args.push(value.clone());
    }
    if let Some(ref param) = params.param {
        args.push("-p".to_string());
        args.push(param.clone());
    }
    if params.all_params {
        args.push("-a".to_string());
    }
    if let Some(ref output) = params.output {
        args.push("-o".to_string());
        args.push(output.clone());
    }
    if params.json {
        args.push("-j".to_string());
    }
    if params.silent {
        args.push("-s".to_string());
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
            let params = normalize_qsreplace_input(&payload).expect("normalize");
            assert_eq!(params.input, "urls.txt");
        }
    }
}
