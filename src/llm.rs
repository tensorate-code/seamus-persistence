//! LLM client — HTTP to Ollama on the GX10 fleet.
//!
//! Simple. Blocking. No async. The CIE doesn't need async —
//! it's spike-driven, not event-driven.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// An LLM endpoint.
#[derive(Clone, Debug)]
pub struct LlmEndpoint {
    pub host: String,
    pub port: u16,
    pub model: String,
}

impl LlmEndpoint {
    pub fn new(host: &str, port: u16, model: &str) -> Self {
        Self {
            host: host.to_string(),
            port,
            model: model.to_string(),
        }
    }

    /// Default: Qwen 14B on GX10-1
    pub fn default_qwen() -> Self {
        Self::new("192.168.1.100", 11434, "qwen2.5:14b")
    }

    /// Chat — send a prompt, get a response. Blocking.
    pub fn chat(&self, prompt: &str) -> Result<String, String> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false
        });
        let body_str = body.to_string();

        let request = format!(
            "POST /api/generate HTTP/1.1\r\n\
             Host: {}:{}\r\n\
             Content-Type: application/json\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
            self.host, self.port, body_str.len(), body_str
        );

        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr.parse().map_err(|e| format!("bad address: {}", e))?,
            Duration::from_secs(10),
        ).map_err(|e| format!("connect failed: {}", e))?;

        stream.set_read_timeout(Some(Duration::from_secs(60)))
            .map_err(|e| format!("set timeout: {}", e))?;

        stream.write_all(request.as_bytes())
            .map_err(|e| format!("write failed: {}", e))?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response)
            .map_err(|e| format!("read failed: {}", e))?;

        let response_str = String::from_utf8_lossy(&response);

        // Extract body (after \r\n\r\n)
        let body = if let Some(idx) = response_str.find("\r\n\r\n") {
            &response_str[idx + 4..]
        } else {
            &response_str
        };

        // Parse JSON response
        let json: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("json parse failed: {} (body: {})", e, &body[..body.len().min(200)]))?;

        json.get("response")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("no response field in: {}", &body[..body.len().min(200)]))
    }
}
