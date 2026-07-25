use std::sync::{Arc, Mutex};

use axum::{
    extract::{RawQuery, State as AxumState},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tauri::Manager;
use tokio::sync::oneshot;
use url::Url;

use super::{normalize_device_code, EvaosTeamsState};

pub(super) struct LoginCallback {
    expected_state: String,
    expected_code: String,
    sender: Mutex<Option<oneshot::Sender<Result<String, String>>>>,
}

impl LoginCallback {
    pub(super) fn new(
        expected_state: String,
        expected_code: String,
        sender: oneshot::Sender<Result<String, String>>,
    ) -> Self {
        Self {
            expected_state,
            expected_code,
            sender: Mutex::new(Some(sender)),
        }
    }

    pub(super) fn try_complete(&self, query: &[(String, String)]) -> Result<(), String> {
        let code = callback_device_code(query, &self.expected_state, &self.expected_code)?;
        let sender = self
            .sender
            .lock()
            .map_err(|error| error.to_string())?
            .take()
            .ok_or_else(|| "Authentication callback was already completed".to_string())?;
        let _ = sender.send(Ok(code));
        Ok(())
    }
}

pub(super) fn callback_device_code(
    query: &[(String, String)],
    expected_state: &str,
    expected_code: &str,
) -> Result<String, String> {
    if query.len() != 2 {
        return Err("Authentication callback contained unexpected fields".to_string());
    }
    let states: Vec<&str> = query
        .iter()
        .filter_map(|(key, value)| (key == "desktop_auth_state").then_some(value.as_str()))
        .collect();
    let codes: Vec<&str> = query
        .iter()
        .filter_map(|(key, value)| (key == "device_code").then_some(value.as_str()))
        .collect();
    let received_state = (states.len() == 1).then_some(states[0]);
    let received_code = (codes.len() == 1).then_some(codes[0]);
    match (received_state, received_code) {
        (Some(received_state), Some(received_code))
            if received_state == expected_state
                && normalize_device_code(received_code) == expected_code =>
        {
            Ok(expected_code.to_string())
        }
        _ => Err("Authentication callback did not match this login attempt".to_string()),
    }
}

pub(super) async fn login_callback(
    RawQuery(raw_query): RawQuery,
    AxumState(state): AxumState<Arc<LoginCallback>>,
) -> Response {
    let query = raw_query
        .as_deref()
        .map(|raw| {
            url::form_urlencoded::parse(raw.as_bytes())
                .into_owned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    match state.try_complete(&query) {
        Ok(()) => (
            StatusCode::OK,
            Html("<!doctype html><title>Hive</title><p>Sign-in received. Return to Hive.</p>"),
        )
            .into_response(),
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Html("<!doctype html><title>Hive</title><p>This sign-in callback is not valid.</p>"),
        )
            .into_response(),
    }
}

#[cfg(feature = "evaos-teams-managed")]
pub(super) fn managed_login_callback_url(url: &Url) -> bool {
    url.scheme() == "evaos-teams"
        && url.host_str() == Some("auth")
        && url.path() == "/callback"
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
}

#[cfg(feature = "evaos-teams-managed")]
pub(crate) fn handle_login_deep_link(app: &tauri::AppHandle, url: &Url) -> bool {
    if !managed_login_callback_url(url) {
        return false;
    }
    let query = url.query_pairs().into_owned().collect::<Vec<_>>();
    let state = app.state::<EvaosTeamsState>();
    let pending = state
        .pending_login
        .lock()
        .ok()
        .and_then(|pending| pending.clone());
    pending.is_some_and(|pending| pending.try_complete(&query).is_ok())
}

#[cfg(feature = "evaos-teams-managed")]
pub(super) fn clear_pending_login(state: &EvaosTeamsState, attempt: &Arc<LoginCallback>) {
    if let Ok(mut pending) = state.pending_login.lock() {
        if pending
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, attempt))
        {
            *pending = None;
        }
    }
}
