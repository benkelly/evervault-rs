use anyhow::{anyhow, Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::{json, Value};

const API_BASE: &str = "https://api.evervault.com";

pub struct EvervaultClient {
    http: Client,
    auth_header: String,
    base_url: String,
}

impl EvervaultClient {
    pub fn new(app_id: &str, api_key: &str) -> Result<Self> {
        Self::with_base_url(app_id, api_key, API_BASE)
    }

    pub fn with_base_url(app_id: &str, api_key: &str, base_url: &str) -> Result<Self> {
        Ok(Self {
            http: Client::builder()
                .build()
                .context("failed to build HTTP client")?,
            auth_header: build_auth_header(app_id, api_key),
            base_url: base_url.trim_end_matches('/').to_string(),
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
        let url = format!("{}{path}", self.base_url);
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

        serde_json::from_str::<Value>(&raw)
            .with_context(|| format!("failed to parse JSON from {url}; raw response: {raw}"))
    }
}

fn build_auth_header(app_id: &str, api_key: &str) -> String {
    let token = BASE64.encode(format!("{app_id}:{api_key}"));
    format!("Basic {token}")
}

fn extract_field(response: &Value, field: &str) -> Result<String> {
    response
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("expected string field `{field}` in response, got: {response}"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_auth_header_encodes_basic_credentials() {
        assert_eq!(build_auth_header("app", "key"), "Basic YXBwOmtleQ==");
    }

    #[test]
    fn build_auth_header_handles_colons_in_api_key() {
        let header = build_auth_header("app_id", "key:with:colons");
        let encoded = header.strip_prefix("Basic ").expect("Basic prefix");
        let decoded = BASE64.decode(encoded).expect("valid base64");
        assert_eq!(decoded, b"app_id:key:with:colons");
    }

    #[test]
    fn build_auth_header_handles_unicode() {
        let header = build_auth_header("app", "ké🔑");
        let encoded = header.strip_prefix("Basic ").expect("Basic prefix");
        let decoded = BASE64.decode(encoded).expect("valid base64");
        assert_eq!(decoded, "app:ké🔑".as_bytes());
    }

    #[test]
    fn extract_field_returns_string_value() {
        let response = json!({ "card_number": "ev:debug:abc" });
        let value = extract_field(&response, "card_number").unwrap();
        assert_eq!(value, "ev:debug:abc");
    }

    #[test]
    fn extract_field_errors_when_field_missing() {
        let response = json!({ "other": "value" });
        let err = extract_field(&response, "card_number").unwrap_err();
        assert!(err.to_string().contains("card_number"));
    }

    #[test]
    fn extract_field_errors_when_value_is_not_a_string() {
        for non_string in [
            json!(42),
            json!(null),
            json!({"nested": "obj"}),
            json!([1, 2]),
        ] {
            let response = json!({ "card_number": non_string });
            let err = extract_field(&response, "card_number").unwrap_err();
            assert!(err.to_string().contains("expected string field"));
        }
    }

    #[test]
    fn api_error_decrypt_forbidden_mentions_permission() {
        let err = api_error("/decrypt", StatusCode::FORBIDDEN, "{}");
        let msg = err.to_string();
        assert!(msg.contains("decrypt"), "got: {msg}");
        assert!(msg.contains("permission"), "got: {msg}");
    }

    #[test]
    fn api_error_unauthorized_mentions_credentials() {
        let err = api_error("/encrypt", StatusCode::UNAUTHORIZED, "{}");
        let msg = err.to_string();
        assert!(msg.contains("EV_APP_ID"), "got: {msg}");
        assert!(msg.contains("EV_API_KEY"), "got: {msg}");
    }

    #[test]
    fn api_error_decrypt_unauthorized_uses_auth_branch_not_decrypt() {
        // Order matters: a 401 on /decrypt should hit the auth-credentials hint,
        // not the decrypt-permission hint (which only applies to 403).
        let err = api_error("/decrypt", StatusCode::UNAUTHORIZED, "{}");
        let msg = err.to_string();
        assert!(msg.contains("EV_APP_ID"), "got: {msg}");
        assert!(!msg.contains("permission"), "got: {msg}");
    }

    #[test]
    fn api_error_encrypt_forbidden_falls_through_to_generic() {
        let err = api_error("/encrypt", StatusCode::FORBIDDEN, "denied");
        let msg = err.to_string();
        assert!(msg.contains("/encrypt"), "got: {msg}");
        assert!(msg.contains("403"), "got: {msg}");
        assert!(msg.contains("denied"), "got: {msg}");
    }

    #[test]
    fn api_error_generic_includes_status_path_and_body() {
        let err = api_error("/encrypt", StatusCode::INTERNAL_SERVER_ERROR, "boom");
        let msg = err.to_string();
        assert!(msg.contains("500"), "got: {msg}");
        assert!(msg.contains("/encrypt"), "got: {msg}");
        assert!(msg.contains("boom"), "got: {msg}");
    }
}

#[cfg(test)]
mod wiremock_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{body_json, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn run_blocking<F, T>(f: F) -> T
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(f).await.unwrap()
    }

    fn client(base_url: &str) -> EvervaultClient {
        EvervaultClient::with_base_url("app_id", "api_key", base_url).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn encrypt_returns_value_from_matching_field() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/encrypt"))
            .and(header("authorization", "Basic YXBwX2lkOmFwaV9rZXk="))
            .and(header("content-type", "application/json"))
            .and(body_json(json!({ "card_number": "4242424242424242" })))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "card_number": "ev:debug:abc" })),
            )
            .expect(1)
            .mount(&server)
            .await;

        let url = server.uri();
        let result = run_blocking(move || client(&url).encrypt("card_number", "4242424242424242"))
            .await
            .unwrap();
        assert_eq!(result, "ev:debug:abc");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn decrypt_returns_value_from_matching_field() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/decrypt"))
            .and(body_json(json!({ "ssn": "ev:debug:xyz" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ssn": "123-45-6789" })))
            .expect(1)
            .mount(&server)
            .await;

        let url = server.uri();
        let result = run_blocking(move || client(&url).decrypt("ssn", "ev:debug:xyz"))
            .await
            .unwrap();
        assert_eq!(result, "123-45-6789");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unauthorized_response_returns_credentials_hint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/encrypt"))
            .respond_with(ResponseTemplate::new(401).set_body_string("invalid api key"))
            .mount(&server)
            .await;

        let url = server.uri();
        let err = run_blocking(move || client(&url).encrypt("field", "value"))
            .await
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("EV_APP_ID"), "got: {msg}");
        assert!(msg.contains("invalid api key"), "got: {msg}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn forbidden_decrypt_returns_permission_hint() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/decrypt"))
            .respond_with(ResponseTemplate::new(403).set_body_string("missing decrypt scope"))
            .mount(&server)
            .await;

        let url = server.uri();
        let err = run_blocking(move || client(&url).decrypt("field", "ev:debug:abc"))
            .await
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("permission"), "got: {msg}");
        assert!(msg.contains("missing decrypt scope"), "got: {msg}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn server_error_returns_generic_error_with_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/encrypt"))
            .respond_with(ResponseTemplate::new(500).set_body_string("upstream blew up"))
            .mount(&server)
            .await;

        let url = server.uri();
        let err = run_blocking(move || client(&url).encrypt("field", "value"))
            .await
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("500"), "got: {msg}");
        assert!(msg.contains("/encrypt"), "got: {msg}");
        assert!(msg.contains("upstream blew up"), "got: {msg}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn malformed_json_response_includes_raw_body_in_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/encrypt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string("not json at all"),
            )
            .mount(&server)
            .await;

        let url = server.uri();
        let err = run_blocking(move || client(&url).encrypt("field", "value"))
            .await
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("failed to parse JSON"), "got: {msg}");
        assert!(msg.contains("not json at all"), "got: {msg}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn missing_field_in_response_returns_helpful_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/encrypt"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "wrong_field": "value" })),
            )
            .mount(&server)
            .await;

        let url = server.uri();
        let err = run_blocking(move || client(&url).encrypt("expected_field", "value"))
            .await
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("expected_field"), "got: {msg}");
    }
}
