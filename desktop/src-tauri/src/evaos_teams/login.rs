use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Query, State as AxumState},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use tokio::sync::oneshot;

use super::{normalize_device_code, EvaosTeamsState, Mutex};

pub(super) struct LoginCallback {
    pub(super) expected_state: String,
    pub(super) sender: Mutex<Option<oneshot::Sender<Result<String, String>>>>,
}

pub(super) struct PendingLoginRegistration<'a> {
    state: &'a EvaosTeamsState,
    callback: Arc<LoginCallback>,
}

impl Drop for PendingLoginRegistration<'_> {
    fn drop(&mut self) {
        if let Ok(mut pending) = self.state.pending_login.lock() {
            if pending
                .as_ref()
                .is_some_and(|callback| Arc::ptr_eq(callback, &self.callback))
            {
                pending.take();
            }
        }
    }
}

pub(super) fn register_pending_login<'a>(
    state: &'a EvaosTeamsState,
    callback: Arc<LoginCallback>,
) -> Result<PendingLoginRegistration<'a>, String> {
    let mut pending = state
        .pending_login
        .lock()
        .map_err(|_| "Hive sign-in state is unavailable".to_string())?;
    if pending.is_some() {
        return Err("Hive sign-in is already in progress".to_string());
    }
    *pending = Some(callback.clone());
    Ok(PendingLoginRegistration { state, callback })
}

fn validated_device_code(value: &str) -> Result<String, String> {
    let normalized = normalize_device_code(value);
    if (8..=40).contains(&normalized.len()) {
        Ok(normalized)
    } else {
        Err("Enter the complete Hive backup code".to_string())
    }
}

fn deliver_login_code(callback: &LoginCallback, code: String) -> Result<(), String> {
    let sender = callback
        .sender
        .lock()
        .map_err(|_| "Hive sign-in state is unavailable".to_string())?
        .take()
        .ok_or_else(|| "This Hive sign-in attempt has already completed".to_string())?;
    sender
        .send(Ok(code))
        .map_err(|_| "This Hive sign-in attempt is no longer active".to_string())
}

pub(super) fn submit_pending_login_code(
    state: &EvaosTeamsState,
    device_code: &str,
) -> Result<(), String> {
    let code = validated_device_code(device_code)?;
    let callback = state
        .pending_login
        .lock()
        .map_err(|_| "Hive sign-in state is unavailable".to_string())?
        .clone()
        .ok_or_else(|| "Start Hive sign-in before entering a backup code".to_string())?;
    deliver_login_code(&callback, code)
}

pub(super) fn callback_device_code(
    query: &HashMap<String, String>,
    expected_state: &str,
) -> Result<String, String> {
    let received_state = query.get("desktop_auth_state").map(String::as_str);
    let received_code = query.get("device_code").map(String::as_str);
    if received_state != Some(expected_state) {
        return Err("Authentication callback did not match this login attempt".to_string());
    }
    if let Some(received_code) = received_code {
        return validated_device_code(received_code)
            .map_err(|_| "Authentication callback did not match this login attempt".to_string());
    }
    Err("Authentication callback did not match this login attempt".to_string())
}

pub(super) async fn login_callback(
    Query(query): Query<HashMap<String, String>>,
    AxumState(state): AxumState<Arc<LoginCallback>>,
) -> Response {
    let result = callback_device_code(&query, &state.expected_state);
    match result {
        Ok(code) => match deliver_login_code(&state, code) {
            Ok(()) => (
                StatusCode::OK,
                Html("<!doctype html><title>Hive</title><p>Sign-in received. Return to Hive.</p>"),
            )
                .into_response(),
            Err(_) => (
                StatusCode::CONFLICT,
                Html("<!doctype html><title>Hive</title><p>This sign-in attempt has already completed.</p>"),
            )
                .into_response(),
        },
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Html("<!doctype html><title>Hive</title><p>This sign-in callback is not valid.</p>"),
        )
            .into_response(),
    }
}
