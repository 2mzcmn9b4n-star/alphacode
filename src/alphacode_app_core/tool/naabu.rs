use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

const DEFAULT_THREADS: usize = 50;
const DEFAULT_RATE: u64 = 1000;

pub struct NaabuTool;

#[derive(Deserialize)]
struct NaabuInput {
    #[serde(default)]
    target: Vec<String>,
    #[serde(default)]
    list: Option<String>,
    /// Ports to scan (e.g., "80,443,8080" or "1-65535")
    #[serde(default)]
    ports: Option<String>,
    /// Scan top N ports
    #[serde(default)]
    top_ports: Option<u32>,
    /// Rate limit
    #[serde(default)]
    rate: Option<u64>,
    /// Threads
    #[serde(default)]
    threads: Option<usize>,
    /// JSON output
    #[serde(default)]
    json: bool,
    /// Silent
    #[serde(default)]
    silent: bool,
    /// Scan all IPs
    #[serde(default)]
    scan_all: bool,
}

#[async_trait]
impl Tool for NaabuTool {
    fn name(&self) -> &str {
        "naabu"
    }

    fn description(&self) -> &str {
        "Fast port scanner. Use AFTER subdomain enumeration to find open ports on discovered hosts. Chains well after subfinder/httpx."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "intent": super::intent_schema_property(),
                "target": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Target hosts to scan."
                },
                "list": {
                    "type": "string",
                    "description": "Path to file with target hosts (one per line)."
                },
                "ports": {
                    "type": "string",
                    "description": "Ports to scan (e.g., '80,443,8080' or '1-65535')."
                },
                "top_ports": {
                    "type": "integer",
                    "description": "Scan top N ports (e.g., 100, 1000)."
                },
                "rate": {
                    "type": "integer",
                    "description": "Packets per second. Default: 1000."
                },
                "threads": {
                    "type": "integer",
                    "description": "Concurrent threads. Default: 50."
                },
                "silent": {
                    "type": "boolean",
                    "description": "Silent mode (only results). Default: false."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: NaabuInput = normalize_naabu_input(&input)?;
        if params.target.is_empty() && params.list.is_none() {
            return Err(anyhow::anyhow!(
                "naabu needs targets: provide `target` (array of hosts) or `list` \
                 (path to a file with one host per line)"
            ));
        }
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "naabu",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("naabu"))
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
            return Err(anyhow::anyhow!("naabu exited with error: {detail}"));
        }

        let (lines, total, truncated) = super::recon_common::parse_lines(&output.stdout);

        let mut result = format!("naabu found {} open ports:\n\n", total);
        for line in &lines {
            result.push_str(line);
            result.push('\n');
        }
        result.push_str(&super::recon_common::truncation_notice(lines.len(), total));

        let mut metadata = HashMap::new();
        metadata.insert("count".to_string(), json!(lines.len()));
        metadata.insert("total_ports".to_string(), json!(total));
        metadata.insert("truncated".to_string(), json!(truncated));

        Ok(ToolOutput::new(result)
            .with_title(format!("naabu: {total} open ports"))
            .with_metadata(json!(metadata)))
    }
}

impl NaabuTool {
    pub fn new() -> Self {
        Self
    }
}

fn normalize_naabu_input(input: &Value) -> Result<NaabuInput> {
    let mut params: NaabuInput = serde_json::from_value(input.clone()).unwrap_or(NaabuInput {
        target: Vec::new(),
        list: None,
        ports: None,
        top_ports: None,
        rate: None,
        threads: None,
        json: false,
        silent: false,
        scan_all: false,
    });
    if !params.target.is_empty() {
        return Ok(params);
    }
    let obj = match input.as_object() {
        Some(obj) => obj,
        None => return Ok(params),
    };
    let mut extra: Vec<String> = Vec::new();
    for key in ["target", "targets", "host", "hosts", "h"] {
        if let Some(value) = obj.get(key) {
            if let Some(s) = value.as_str() {
                if !s.trim().is_empty() {
                    extra.push(s.trim().to_string());
                }
            } else if let Some(arr) = value.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str()
                        && !s.trim().is_empty()
                    {
                        extra.push(s.trim().to_string());
                    }
                }
            }
        }
    }
    params.target.extend(extra);
    Ok(params)
}

fn build_args(params: &NaabuInput) -> Result<Vec<String>> {
    let mut args = Vec::new();

    if let Some(ref list) = params.list {
        args.push("-l".to_string());
        args.push(super::recon_common::validate_file_arg(list, "list")?);
    } else if !params.target.is_empty() {
        for target in &params.target {
            args.push("-host".to_string());
            args.push(super::recon_common::validate_target(target)?);
        }
    }

    if let Some(ref ports) = params.ports {
        args.push("-p".to_string());
        args.push(ports.clone());
    }
    if let Some(top) = params.top_ports {
        args.push("-tp".to_string());
        args.push(top.to_string());
    }
    if params.scan_all {
        args.push("-sa".to_string());
    }
    if params.silent {
        args.push("-silent".to_string());
    }
    if params.json {
        args.push("-json".to_string());
    }

    let rate = params.rate.unwrap_or(DEFAULT_RATE);
    args.push("-rate".to_string());
    args.push(rate.to_string());

    let threads = params.threads.unwrap_or(DEFAULT_THREADS);
    args.push("-c".to_string());
    args.push(threads.to_string());

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_accepts_aliases() {
        for payload in [
            serde_json::json!({"target": "example.com"}),
            serde_json::json!({"host": "example.com"}),
        ] {
            let params = normalize_naabu_input(&payload).expect("normalize");
            assert!(!params.target.is_empty());
        }
    }
}
