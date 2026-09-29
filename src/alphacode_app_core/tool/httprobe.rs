use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub struct HttprobeTool;

#[derive(Deserialize)]
struct HttprobeInput {
    /// Input file or stdin
    input: String,
    /// Ports to probe
    #[serde(default)]
    ports: Option<String>,
    /// HTTPS only
    #[serde(default)]
    https_only: bool,
    /// HTTP only
    #[serde(default)]
    http_only: bool,
    /// Follow redirects
    #[serde(default)]
    follow_redirects: bool,
    /// Timeout
    #[serde(default)]
    timeout: Option<u64>,
    /// Concurrency
    #[serde(default)]
    concurrency: Option<usize>,
    /// Output file
    #[serde(default)]
    output: Option<String>,
    /// JSON output
    #[serde(default)]
    json: bool,
    /// Silent
    #[serde(default)]
    silent: bool,
    /// Verbose
    #[serde(default)]
    verbose: bool,
    /// Status codes
    #[serde(default)]
    status_codes: Option<String>,
    /// Title
    #[serde(default)]
    title: bool,
    /// Tech detect
    #[serde(default)]
    tech_detect: bool,
    /// Content length
    #[serde(default)]
    content_length: bool,
    /// Web server
    #[serde(default)]
    web_server: bool,
    /// CDN
    #[serde(default)]
    cdn: bool,
    /// TLS
    #[serde(default)]
    tls: bool,
    /// Method
    #[serde(default)]
    method: bool,
    /// Response size
    #[serde(default)]
    response_size: bool,
    /// Content type
    #[serde(default)]
    content_type: bool,
    /// Location
    #[serde(default)]
    location: bool,
    /// Body hash
    #[serde(default)]
    body_hash: bool,
    /// Header hash
    #[serde(default)]
    header_hash: bool,
    /// Favicon hash
    #[serde(default)]
    favicon_hash: bool,
    /// Screenshot
    #[serde(default)]
    screenshot: bool,
    /// Save response
    #[serde(default)]
    save_response: bool,
    /// Save response dir
    #[serde(default)]
    save_response_dir: Option<String>,
    /// Save response file
    #[serde(default)]
    save_response_file: Option<String>,
    /// Save response header
    #[serde(default)]
    save_response_header: bool,
    /// Save response body
    #[serde(default)]
    save_response_body: bool,
    /// Save response body file
    #[serde(default)]
    save_response_body_file: Option<String>,
    /// Save response body dir
    #[serde(default)]
    save_response_body_dir: Option<String>,
    /// Save response header file
    #[serde(default)]
    save_response_header_file: Option<String>,
    /// Save response header dir
    #[serde(default)]
    save_response_header_dir: Option<String>,
    /// Save response screenshot file
    #[serde(default)]
    save_response_screenshot_file: Option<String>,
    /// Save response screenshot dir
    #[serde(default)]
    save_response_screenshot_dir: Option<String>,
    /// save response body hash file
    #[serde(default)]
    save_response_body_hash_file: Option<String>,
    /// save response body hash dir
    #[serde(default)]
    save_response_body_hash_dir: Option<String>,
    /// save response header hash file
    #[serde(default)]
    save_response_header_hash_file: Option<String>,
    /// save response header hash dir
    #[serde(default)]
    save_response_header_hash_dir: Option<String>,
    /// save response favicon hash file
    #[serde(default)]
    save_response_favicon_hash_file: Option<String>,
    /// save response favicon hash dir
    #[serde(default)]
    save_response_favicon_hash_dir: Option<String>,
}

#[async_trait]
impl Tool for HttprobeTool {
    fn name(&self) -> &str {
        "httprobe"
    }

    fn description(&self) -> &str {
        "Probe live hosts. Use to check which hosts are live and responding. Chains well after subdomain enumeration."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["input"],
            "properties": {
                "intent": super::intent_schema_property(),
                "input": {
                    "type": "string",
                    "description": "Input file with hosts (one per line)."
                },
                "ports": {
                    "type": "string",
                    "description": "Ports to probe (e.g., '80,443,8080')."
                },
                "https_only": {
                    "type": "boolean",
                    "description": "HTTPS only. Default: false."
                },
                "http_only": {
                    "type": "boolean",
                    "description": "HTTP only. Default: false."
                },
                "follow_redirects": {
                    "type": "boolean",
                    "description": "Follow redirects. Default: false."
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds."
                },
                "concurrency": {
                    "type": "integer",
                    "description": "Concurrent requests. Default: 50."
                },
                "title": {
                    "type": "boolean",
                    "description": "Extract page title. Default: false."
                },
                "tech_detect": {
                    "type": "boolean",
                    "description": "Detect technology stack. Default: false."
                }
            }
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: HttprobeInput = normalize_httprobe_input(&input)?;
        let args = build_args(&params)?;

        let output = super::recon_common::run_bounded(
            "httprobe",
            &args,
            super::recon_common::DEFAULT_TOOL_TIMEOUT,
        )
        .await
        .map_err(|e| {
            if e.starts_with("failed to run") {
                anyhow::anyhow!("{e}. {}", super::recon_common::install_hint("httprobe"))
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
            return Err(anyhow::anyhow!("httprobe exited with error: {detail}"));
        }

        let (lines, total, truncated) = super::recon_common::parse_lines(&output.stdout);

        let mut result = format!("httprobe found {} live hosts:\n\n", total);
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
            .with_title(format!("httprobe: {total} live hosts"))
            .with_metadata(json!(metadata)))
    }
}

impl HttprobeTool {
    pub fn new() -> Self {
        Self
    }
}

fn normalize_httprobe_input(input: &Value) -> Result<HttprobeInput> {
    if let Ok(params) = serde_json::from_value::<HttprobeInput>(input.clone()) {
        return Ok(params);
    }
    let obj = input.as_object().ok_or_else(|| {
        anyhow::anyhow!(
            "httprobe expects a JSON object with `input`, e.g. {{\"input\": \"hosts.txt\"}}"
        )
    })?;
    let input_val = ["input", "file", "i", "hosts"]
        .iter()
        .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing field `input`. Provide the input as `input`."))?;
    Ok(HttprobeInput {
        input: input_val.to_string(),
        ports: obj.get("ports").and_then(|v| v.as_str()).map(String::from),
        https_only: obj
            .get("https_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        http_only: obj
            .get("http_only")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        follow_redirects: obj
            .get("follow_redirects")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        timeout: obj.get("timeout").and_then(|v| v.as_u64()),
        concurrency: obj
            .get("concurrency")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        output: obj.get("output").and_then(|v| v.as_str()).map(String::from),
        json: obj.get("json").and_then(|v| v.as_bool()).unwrap_or(false),
        silent: obj.get("silent").and_then(|v| v.as_bool()).unwrap_or(false),
        verbose: obj
            .get("verbose")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        status_codes: obj
            .get("status_codes")
            .and_then(|v| v.as_str())
            .map(String::from),
        title: obj.get("title").and_then(|v| v.as_bool()).unwrap_or(false),
        tech_detect: obj
            .get("tech_detect")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        content_length: obj
            .get("content_length")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        web_server: obj
            .get("web_server")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        cdn: obj.get("cdn").and_then(|v| v.as_bool()).unwrap_or(false),
        tls: obj.get("tls").and_then(|v| v.as_bool()).unwrap_or(false),
        method: obj.get("method").and_then(|v| v.as_bool()).unwrap_or(false),
        response_size: obj
            .get("response_size")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        content_type: obj
            .get("content_type")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        location: obj
            .get("location")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        body_hash: obj
            .get("body_hash")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        header_hash: obj
            .get("header_hash")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        favicon_hash: obj
            .get("favicon_hash")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        screenshot: obj
            .get("screenshot")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        save_response: obj
            .get("save_response")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        save_response_dir: obj
            .get("save_response_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_file: obj
            .get("save_response_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_header: obj
            .get("save_response_header")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        save_response_body: obj
            .get("save_response_body")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        save_response_body_file: obj
            .get("save_response_body_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_body_dir: obj
            .get("save_response_body_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_header_file: obj
            .get("save_response_header_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_header_dir: obj
            .get("save_response_header_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_screenshot_file: obj
            .get("save_response_screenshot_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_screenshot_dir: obj
            .get("save_response_screenshot_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_body_hash_file: obj
            .get("save_response_body_hash_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_body_hash_dir: obj
            .get("save_response_body_hash_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_header_hash_file: obj
            .get("save_response_header_hash_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_header_hash_dir: obj
            .get("save_response_header_hash_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_favicon_hash_file: obj
            .get("save_response_favicon_hash_file")
            .and_then(|v| v.as_str())
            .map(String::from),
        save_response_favicon_hash_dir: obj
            .get("save_response_favicon_hash_dir")
            .and_then(|v| v.as_str())
            .map(String::from),
    })
}

fn build_args(params: &HttprobeInput) -> Result<Vec<String>> {
    let mut args = Vec::new();

    args.push(params.input.clone());

    if let Some(ref ports) = params.ports {
        args.push("-p".to_string());
        args.push(ports.clone());
    }
    if params.https_only {
        args.push("-https".to_string());
    }
    if params.http_only {
        args.push("-http".to_string());
    }
    if params.follow_redirects {
        args.push("-follow-redirects".to_string());
    }
    if let Some(timeout) = params.timeout {
        args.push("-timeout".to_string());
        args.push(timeout.to_string());
    }
    if let Some(concurrency) = params.concurrency {
        args.push("-c".to_string());
        args.push(concurrency.to_string());
    }
    if let Some(ref output) = params.output {
        args.push("-o".to_string());
        args.push(output.clone());
    }
    if params.json {
        args.push("-json".to_string());
    }
    if params.silent {
        args.push("-silent".to_string());
    }
    if params.verbose {
        args.push("-verbose".to_string());
    }
    if let Some(ref codes) = params.status_codes {
        args.push("-status-code".to_string());
        args.push(codes.clone());
    }
    if params.title {
        args.push("-title".to_string());
    }
    if params.tech_detect {
        args.push("-tech-detect".to_string());
    }
    if params.content_length {
        args.push("-content-length".to_string());
    }
    if params.web_server {
        args.push("-web-server".to_string());
    }
    if params.cdn {
        args.push("-cdn".to_string());
    }
    if params.tls {
        args.push("-tls".to_string());
    }
    if params.method {
        args.push("-method".to_string());
    }
    if params.response_size {
        args.push("-response-size".to_string());
    }
    if params.content_type {
        args.push("-content-type".to_string());
    }
    if params.location {
        args.push("-location".to_string());
    }
    if params.body_hash {
        args.push("-body-hash".to_string());
    }
    if params.header_hash {
        args.push("-header-hash".to_string());
    }
    if params.favicon_hash {
        args.push("-favicon-hash".to_string());
    }
    if params.screenshot {
        args.push("-screenshot".to_string());
    }
    if params.save_response {
        args.push("-save-response".to_string());
    }
    if let Some(ref dir) = params.save_response_dir {
        args.push("-save-response-dir".to_string());
        args.push(dir.clone());
    }
    if let Some(ref file) = params.save_response_file {
        args.push("-save-response-file".to_string());
        args.push(file.clone());
    }
    if params.save_response_header {
        args.push("-save-response-header".to_string());
    }
    if params.save_response_body {
        args.push("-save-response-body".to_string());
    }
    if let Some(ref file) = params.save_response_body_file {
        args.push("-save-response-body-file".to_string());
        args.push(file.clone());
    }
    if let Some(ref dir) = params.save_response_body_dir {
        args.push("-save-response-body-dir".to_string());
        args.push(dir.clone());
    }
    if let Some(ref file) = params.save_response_header_file {
        args.push("-save-response-header-file".to_string());
        args.push(file.clone());
    }
    if let Some(ref dir) = params.save_response_header_dir {
        args.push("-save-response-header-dir".to_string());
        args.push(dir.clone());
    }
    if let Some(ref file) = params.save_response_screenshot_file {
        args.push("-save-response-screenshot-file".to_string());
        args.push(file.clone());
    }
    if let Some(ref dir) = params.save_response_screenshot_dir {
        args.push("-save-response-screenshot-dir".to_string());
        args.push(dir.clone());
    }
    if let Some(ref file) = params.save_response_body_hash_file {
        args.push("-save-response-body-hash-file".to_string());
        args.push(file.clone());
    }
    if let Some(ref dir) = params.save_response_body_hash_dir {
        args.push("-save-response-body-hash-dir".to_string());
        args.push(dir.clone());
    }
    if let Some(ref file) = params.save_response_header_hash_file {
        args.push("-save-response-header-hash-file".to_string());
        args.push(file.clone());
    }
    if let Some(ref dir) = params.save_response_header_hash_dir {
        args.push("-save-response-header-hash-dir".to_string());
        args.push(dir.clone());
    }
    if let Some(ref file) = params.save_response_favicon_hash_file {
        args.push("-save-response-favicon-hash-file".to_string());
        args.push(file.clone());
    }
    if let Some(ref dir) = params.save_response_favicon_hash_dir {
        args.push("-save-response-favicon-hash-dir".to_string());
        args.push(dir.clone());
    }

    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_accepts_aliases() {
        for payload in [
            serde_json::json!({"input": "hosts.txt"}),
            serde_json::json!({"file": "hosts.txt"}),
        ] {
            let params = normalize_httprobe_input(&payload).expect("normalize");
            assert_eq!(params.input, "hosts.txt");
        }
    }
}
