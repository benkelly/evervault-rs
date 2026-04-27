use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.evervault.com";

pub struct EvervaultClient {
    http: Client,
    auth_header: String,
}

impl EvervaultClient {
    pub fn new(app_id: &str, api_key: &str) -> Result<Self> {
        let token = BASE64.encode(format!("{app_id}:{api_key}"));
        Ok(Self {
            http: Client::builder()
                .build()
                .context("failed to build HTTP client")?,
            auth_header: format!("Basic {token}"),
        })
    }

    pub fn encrypt(&self, field: &str, value: &str) -> Result<String> {
        let body = json!({ field: value });
        let response = self.post("/encrypt", &body)?;
        extract_field(&response, field)
    }

    pub fn decrypt(&self, field: &str, token: &str) -> Result<String> {
        let body = json!({ field: token });
        let response = self.post("/decrypt", &body)?;
        extract_field(&response, field)
    }

    fn post(&self, path: &str, body: &Value) -> Result<Value> {
        let url = format!("{API_BASE}{path}");
        let response = self
            .http
            .post(&url)
            .header("Authorization", &self.auth_header)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .with_context(|| format!("failed to send request to {url}"))?;

        let status = response.status();
        let raw = response
            .text()
            .with_context(|| format!("failed to read response body from {url}"))?;

        if !status.is_success() {
            return Err(api_error(path, status, &raw));
        }

        serde_json::from_str::<Value>(&raw).with_context(|| {
            format!("failed to parse JSON from {url}; raw response: {raw}")
        })
    }
}

fn extract_field(response: &Value, field: &str) -> Result<String> {
    response
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow!(
                "expected string field `{field}` in response, got: {response}"
            )
        })
}

fn api_error(path: &str, status: StatusCode, body: &str) -> anyhow::Error {
    if path == "/decrypt" && status == StatusCode::FORBIDDEN {
        return anyhow!(
            "decrypt request was forbidden (HTTP 403). \
             Make sure your API key has the `decrypt` permission enabled \
             in the Evervault dashboard. Response body: {body}"
        );
    }

    if status == StatusCode::UNAUTHORIZED {
        return anyhow!(
            "authentication failed (HTTP 401). \
             Check that EV_APP_ID and EV_API_KEY are correct. \
             Response body: {body}"
        );
    }

    anyhow!("Evervault API error from {path}: HTTP {status}. Body: {body}")
}
