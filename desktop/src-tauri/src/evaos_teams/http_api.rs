use std::time::Duration;

use serde::Deserialize;
use url::Url;

const SUPABASE_ORIGIN: &str = "https://rhfojelkgtwcxnrfhtlj.supabase.co";
// The publishable client identifier is injected only into managed builds. It
// is intentionally absent from source control and is not an authorization
// credential; authorization still comes only from the opaque desktop session.
const SUPABASE_PUBLISHABLE_KEY: Option<&str> = option_env!("HIVE_SUPABASE_PUBLISHABLE_KEY");
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(super) struct ApiFailure {
    pub(super) status: reqwest::StatusCode,
    pub(super) code: String,
}

impl std::fmt::Display for ApiFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "HTTP {} ({})", self.status, self.code)
    }
}

impl ApiFailure {
    pub(super) fn means_session_is_absent(&self) -> bool {
        matches!(self.status.as_u16(), 401 | 404)
    }
}

fn error_code_from_value(value: serde_json::Value) -> String {
    value
        .get("error")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 160
                && value.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || byte == b'_'
                        || byte == b'-'
                })
        })
        .unwrap_or("request_failed")
        .to_string()
}

fn functions_url(name: &str) -> Result<Url, String> {
    Url::parse(&format!("{SUPABASE_ORIGIN}/functions/v1/{name}"))
        .map_err(|error| format!("invalid managed API URL: {error}"))
}

pub(super) async fn post_json<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    function: &str,
    bearer: Option<&str>,
    body: serde_json::Value,
) -> Result<T, ApiFailure> {
    let publishable_key = SUPABASE_PUBLISHABLE_KEY
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ApiFailure {
            status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            code: "missing_build_configuration".to_string(),
        })?;
    let url = functions_url(function).map_err(|code| ApiFailure {
        status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        code,
    })?;
    let mut request = client
        .post(url)
        .header("apikey", publishable_key)
        .header(
            "x-client-info",
            format!("hive-desktop/{}", env!("CARGO_PKG_VERSION")),
        )
        .timeout(REQUEST_TIMEOUT)
        .json(&body);
    if let Some(token) = bearer {
        request = request.bearer_auth(token);
    }
    let response = request.send().await.map_err(|error| ApiFailure {
        status: reqwest::StatusCode::SERVICE_UNAVAILABLE,
        code: format!("network_error:{}", error.is_timeout()),
    })?;
    let status = response.status();
    if !status.is_success() {
        let code = response
            .json::<serde_json::Value>()
            .await
            .map(error_code_from_value)
            .unwrap_or_else(|_| "request_failed".to_string());
        return Err(ApiFailure { status, code });
    }
    let value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|_| ApiFailure {
            status,
            code: "invalid_json".to_string(),
        })?;
    serde_json::from_value(value).map_err(|_| ApiFailure {
        status,
        code: "invalid_response".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_accepts_only_bounded_machine_codes() {
        assert_eq!(
            error_code_from_value(serde_json::json!({
                "error": "identity_custody_enrollment_unavailable"
            })),
            "identity_custody_enrollment_unavailable"
        );
        for value in [
            serde_json::json!({"error": "Contains spaces"}),
            serde_json::json!({"error": "UPPERCASE"}),
            serde_json::json!({"error": "x".repeat(161)}),
            serde_json::json!({"message": "not an error code"}),
        ] {
            assert_eq!(error_code_from_value(value), "request_failed");
        }
    }
}
