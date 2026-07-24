use std::time::Duration;

use serde::Deserialize;
use url::Url;

const SUPABASE_ORIGIN: &str = "https://rhfojelkgtwcxnrfhtlj.supabase.co";
// Supabase publishable keys are intentionally public client identifiers. This
// value grants no service-role access; authorization still comes exclusively
// from the opaque Desktop session returned after browser authentication.
const SUPABASE_PUBLISHABLE_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmUiLCJyZWYiOiJyaGZvamVsa2d0d2N4bnJmaHRsaiIsInJvbGUiOiJhbm9uIiwiaWF0IjoxNzczMjQzNTc2LCJleHAiOjIwODg4MTk1NzZ9.X8mJHaYIolCmx6j_473GGb05OyFTy43Hq-BEelZRjAE";
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
    let url = functions_url(function).map_err(|code| ApiFailure {
        status: reqwest::StatusCode::INTERNAL_SERVER_ERROR,
        code,
    })?;
    let mut request = client
        .post(url)
        .header("apikey", SUPABASE_PUBLISHABLE_KEY)
        .header("x-client-info", "evaos-teams-desktop/0.4.23")
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
        return Err(ApiFailure {
            status,
            // A remote error body is untrusted and may echo a device code,
            // signed challenge, or bearer token. Only return a local category.
            code: "request_failed".to_string(),
        });
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
