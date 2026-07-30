use std::{collections::HashMap, sync::Mutex};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tokio::sync::oneshot;

use super::device_code::normalize_device_code;

/// State held by the single-use local OAuth callback listener.
pub(super) struct LoginCallback {
    pub(super) expected_state: String,
    pub(super) sender: Mutex<Option<oneshot::Sender<Result<String, String>>>>,
}

/// Validate callback state and normalize the proof-bound device code.
pub(super) fn callback_device_code(
    query: &HashMap<String, String>,
    expected_state: &str,
) -> Result<String, String> {
    if query.get("desktop_auth_state").map(String::as_str) != Some(expected_state) {
        return Err("Authentication callback did not match this login attempt".to_string());
    }
    if let Some(received_code) = query.get("device_code") {
        let normalized = normalize_device_code(received_code);
        if (8..=40).contains(&normalized.len()) {
            return Ok(normalized);
        }
    }
    Err("Authentication callback did not match this login attempt".to_string())
}

/// Complete the local OAuth callback without exposing session material to the
/// browser.
pub(super) async fn login_callback(
    Query(query): Query<HashMap<String, String>>,
    State(state): State<std::sync::Arc<LoginCallback>>,
) -> Response {
    match callback_device_code(&query, &state.expected_state) {
        Ok(code) => {
            if let Ok(mut sender) = state.sender.lock() {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(Ok(code));
                }
            }
            (
                StatusCode::OK,
                Html("<!doctype html><title>Hive</title><p>Sign-in received. Return to Hive.</p>"),
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Html("<!doctype html><title>Hive</title><p>This sign-in callback is not valid.</p>"),
        )
            .into_response(),
    }
}
