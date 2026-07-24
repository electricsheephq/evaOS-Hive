//! Electric-only evaOS Teams authentication.
//!
//! This module is intentionally separate from Buzz's native identity path.
//! Managed builds use an opaque Electric Desktop session plus a Nostr key held
//! under a distinct OS-keyring service. Neither value is serialized to the
//! renderer, migrated from native Buzz, or accepted from environment input.
#![cfg_attr(not(feature = "evaos-teams-managed"), allow(dead_code, unused_imports))]

use std::{
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use axum::{routing::get, Router};
use nostr::{EventBuilder, Keys, Kind, Tag, Timestamp};
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_opener::OpenerExt;
use tokio::{net::TcpListener, sync::oneshot};
use url::Url;
use zeroize::Zeroizing;

use crate::{app_state::AppState, secret_store::SecretStore};

mod api;
mod credentials;
mod login_callback;
#[cfg(feature = "evaos-teams-managed")]
mod logout;
use api::{post_json, ApiFailure};
#[cfg(feature = "evaos-teams-managed")]
use credentials::managed_credential_entries;
#[cfg(test)]
use login_callback::callback_device_code;
#[cfg(feature = "evaos-teams-managed")]
use login_callback::clear_pending_login;
#[cfg(feature = "evaos-teams-managed")]
pub(crate) use login_callback::handle_login_deep_link;
#[cfg(all(test, feature = "evaos-teams-managed"))]
use login_callback::managed_login_callback_url;
use login_callback::{login_callback, LoginCallback};
#[cfg(feature = "evaos-teams-managed")]
use logout::{begin_managed_logout, retry_pending_logout};

const DASHBOARD_ORIGIN: &str = "https://www.electricsheephq.com";
const KEYRING_SERVICE: &str = "evaos-teams-desktop";
const IDENTITY_KEY: &str = "identity";
const SESSION_KEY: &str = "electric_desktop_session";
const LOGOUT_PENDING_KEY: &str = "logout_pending";
const LOGOUT_CONFIRMED_KEY: &str = "logout_confirmed";
const PREVIOUS_SESSION_KEY: &str = "previous_electric_desktop_session";
const KEY_BINDING_KIND: u16 = 27_235;
const KEY_BINDING_SCHEMA: &str = "evaos.buzz_key_binding.v1";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);

fn managed_store() -> &'static SecretStore {
    static STORE: OnceLock<SecretStore> = OnceLock::new();
    STORE.get_or_init(|| SecretStore::keyring(KEYRING_SERVICE))
}

#[cfg(feature = "evaos-teams-managed")]
fn verify_managed_store_writable() -> Result<(), String> {
    let rewritten = managed_store()
        .force_rewrite_current()
        .map_err(|_| "evaOS Teams cannot write to macOS Keychain".to_string())?;
    if managed_store().load_all_readonly()? != Some(rewritten) {
        return Err("evaOS Teams could not verify its Keychain write".to_string());
    }
    Ok(())
}

/// Public, renderer-safe entitlement projection.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaosTeamsEntitlement {
    community_id: String,
    relay_host: String,
    public_key: Option<String>,
    role: String,
    assignment_status: String,
    reconciliation_status: String,
    access_revision: u64,
    expires_at: String,
    refresh_after_seconds: u64,
}

/// Public authentication state. It deliberately contains no Desktop token,
/// device code, private key, challenge nonce, email, or account selector.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EvaosTeamsAuthStatus {
    managed: bool,
    phase: &'static str,
    authenticated: bool,
    keychain_available: bool,
    message: Option<String>,
    entitlement: Option<EvaosTeamsEntitlement>,
}

impl EvaosTeamsAuthStatus {
    #[cfg(not(feature = "evaos-teams-managed"))]
    fn unmanaged() -> Self {
        Self {
            managed: false,
            phase: "native",
            authenticated: false,
            keychain_available: true,
            message: None,
            entitlement: None,
        }
    }

    fn signed_out() -> Self {
        Self {
            managed: true,
            phase: "signed_out",
            authenticated: false,
            keychain_available: true,
            message: None,
            entitlement: None,
        }
    }

    fn locked(message: String) -> Self {
        Self {
            managed: true,
            phase: "keychain_locked",
            authenticated: false,
            keychain_available: false,
            message: Some(message),
            entitlement: None,
        }
    }

    fn reauth(message: String) -> Self {
        Self {
            managed: true,
            phase: "reauth_required",
            authenticated: false,
            keychain_available: true,
            message: Some(message),
            entitlement: None,
        }
    }

    fn logout_pending(message: String) -> Self {
        Self {
            managed: true,
            phase: "logout_pending",
            authenticated: false,
            keychain_available: true,
            message: Some(message),
            entitlement: None,
        }
    }

    fn active(entitlement: EvaosTeamsEntitlement) -> Self {
        Self {
            managed: true,
            phase: "active",
            authenticated: true,
            keychain_available: true,
            message: None,
            entitlement: Some(entitlement),
        }
    }

    fn sync_pending(entitlement: EvaosTeamsEntitlement, message: String) -> Self {
        Self {
            managed: true,
            phase: "sync_pending",
            authenticated: true,
            keychain_available: true,
            message: Some(message),
            entitlement: Some(entitlement),
        }
    }
}

#[derive(Default)]
struct ManagedRuntime {
    initialized: bool,
    session: Option<Zeroizing<String>>,
    previous_session: Option<Zeroizing<String>>,
    keys: Option<Keys>,
    logout_pending: bool,
    logout_confirmed: bool,
}

/// Backend-only managed credential state.
#[derive(Default)]
pub(crate) struct EvaosTeamsState {
    runtime: Mutex<ManagedRuntime>,
    operation: tokio::sync::Mutex<()>,
    pending_login: Mutex<Option<Arc<LoginCallback>>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct KeyBindingChallenge {
    schema_version: String,
    identity_id: String,
    membership_id: String,
    community_id: String,
    desktop_session_id: String,
    public_key: String,
    nonce: String,
    expires_at: String,
}

#[derive(Debug, Deserialize)]
struct EventTemplate {
    kind: u16,
    created_at: u64,
    tags: Vec<Vec<String>>,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    status: String,
    challenge: KeyBindingChallenge,
    event_template: EventTemplate,
    relay_host: String,
}

#[derive(Debug, Deserialize)]
struct EntitlementResponse {
    status: String,
    entitlement: EvaosTeamsEntitlement,
}

#[derive(Debug, Deserialize)]
struct LogoutResponse {
    status: String,
}

#[derive(Debug, Deserialize)]
struct ClaimResponse {
    desktop_session: String,
    desktop_session_expires_at: String,
}

fn normalize_device_code(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

fn dashboard_login_url(callback: &str, state: &str, device_code: &str) -> Result<Url, String> {
    let mut url = Url::parse(&format!("{DASHBOARD_ORIGIN}/desktop-auth"))
        .map_err(|error| format!("invalid dashboard URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("desktop_callback", callback)
        .append_pair("desktop_auth_state", state)
        .append_pair("callback_scheme", "evaos-teams")
        .append_pair("fresh", device_code)
        .append_pair("switch_account", "1")
        .append_pair("prompt", "select_account");
    Ok(url)
}

fn relay_websocket_url(relay_host: &str) -> Result<String, String> {
    let mut url = Url::parse(relay_host).map_err(|_| "invalid managed relay URL".to_string())?;
    if url.scheme() != "https"
        || url.username() != ""
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
    {
        return Err("managed relay must be a credential-free HTTPS origin".to_string());
    }
    url.set_scheme("wss")
        .map_err(|_| "could not convert managed relay URL".to_string())?;
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn validate_entitlement(
    entitlement: &EvaosTeamsEntitlement,
    expected_public_key: &str,
) -> Result<String, String> {
    uuid::Uuid::parse_str(&entitlement.community_id)
        .map_err(|_| "managed entitlement has an invalid community".to_string())?;
    if entitlement.public_key.as_deref() != Some(expected_public_key) {
        return Err("managed entitlement belongs to a different identity".to_string());
    }
    if !matches!(
        entitlement.role.as_str(),
        "owner" | "admin" | "member" | "employee" | "agent_only"
    ) || !matches!(
        entitlement.assignment_status.as_str(),
        "unassigned" | "pending" | "assigned"
    ) || !matches!(
        entitlement.reconciliation_status.as_str(),
        "pending" | "failed" | "current"
    ) || entitlement.refresh_after_seconds < 30
        || entitlement.refresh_after_seconds > 3600
    {
        return Err("managed entitlement policy is invalid".to_string());
    }
    let expires_at = chrono::DateTime::parse_from_rfc3339(&entitlement.expires_at)
        .map_err(|_| "managed entitlement expiry is invalid".to_string())?;
    if expires_at <= chrono::Utc::now() {
        return Err("managed entitlement has expired".to_string());
    }
    relay_websocket_url(&entitlement.relay_host)
}

fn entitlement_access_ready(entitlement: &EvaosTeamsEntitlement) -> bool {
    entitlement.reconciliation_status == "current"
        && (entitlement.role != "agent_only" || entitlement.assignment_status == "assigned")
}

fn entitlement_sync_message(entitlement: &EvaosTeamsEntitlement) -> String {
    if entitlement.reconciliation_status == "failed" {
        "Managed relay projection failed and access remains disabled while ElectricSheep retries"
            .to_string()
    } else if entitlement.role == "agent_only" && entitlement.assignment_status != "assigned" {
        "This managed agent is waiting for its server assignment; relay access remains disabled"
            .to_string()
    } else {
        "ElectricSheep verified this device; managed relay projection is still being reconciled"
            .to_string()
    }
}

fn bind_verified_entitlement(
    mut entitlement: EvaosTeamsEntitlement,
    expected_relay: &str,
    public_key: &str,
) -> Result<EvaosTeamsEntitlement, String> {
    if entitlement.relay_host != expected_relay || entitlement.public_key.is_some() {
        return Err("Managed key verification changed the server-selected scope".to_string());
    }
    entitlement.public_key = Some(public_key.to_string());
    validate_entitlement(&entitlement, public_key)?;
    Ok(entitlement)
}

fn validate_challenge(
    response: &ChallengeResponse,
    expected_public_key: &str,
) -> Result<(), String> {
    if response.status != "challenge_issued"
        || response.challenge.schema_version != KEY_BINDING_SCHEMA
        || response.challenge.public_key != expected_public_key
        || response.event_template.kind != KEY_BINDING_KIND
    {
        return Err("managed key challenge does not match this device".to_string());
    }
    for id in [
        &response.challenge.identity_id,
        &response.challenge.membership_id,
        &response.challenge.community_id,
        &response.challenge.desktop_session_id,
    ] {
        uuid::Uuid::parse_str(id)
            .map_err(|_| "managed key challenge contains an invalid identifier".to_string())?;
    }
    if response.challenge.nonce.len() != 43
        || !response.challenge.nonce.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return Err("managed key challenge nonce is invalid".to_string());
    }
    let expected_content = serde_json::to_string(&response.challenge)
        .map_err(|error| format!("could not serialize managed challenge: {error}"))?;
    let expected_tags = vec![
        vec!["t".to_string(), "evaos-teams-key-binding".to_string()],
        vec!["challenge".to_string(), response.challenge.nonce.clone()],
    ];
    if response.event_template.content != expected_content
        || response.event_template.tags != expected_tags
    {
        return Err("managed key challenge template is not canonical".to_string());
    }
    let expires_at = chrono::DateTime::parse_from_rfc3339(&response.challenge.expires_at)
        .map_err(|_| "managed key challenge expiry is invalid".to_string())?;
    let now = chrono::Utc::now();
    if expires_at <= now || expires_at > now + chrono::Duration::minutes(5) {
        return Err("managed key challenge has expired".to_string());
    }
    let created_at = i64::try_from(response.event_template.created_at)
        .map_err(|_| "managed key challenge timestamp is invalid".to_string())?;
    if (created_at - now.timestamp()).abs() > 5 * 60 {
        return Err("managed key challenge timestamp is invalid".to_string());
    }
    relay_websocket_url(&response.relay_host)?;
    Ok(())
}

fn signed_challenge(
    response: &ChallengeResponse,
    keys: &Keys,
) -> Result<serde_json::Value, String> {
    validate_challenge(response, &keys.public_key().to_hex())?;
    let tags = response
        .event_template
        .tags
        .iter()
        .cloned()
        .map(|tag| Tag::parse(tag).map_err(|error| format!("invalid challenge tag: {error}")))
        .collect::<Result<Vec<_>, _>>()?;
    let event = EventBuilder::new(
        Kind::Custom(response.event_template.kind),
        response.event_template.content.clone(),
    )
    .tags(tags)
    .custom_created_at(Timestamp::from(response.event_template.created_at))
    .sign_with_keys(keys)
    .map_err(|error| format!("could not sign managed key challenge: {error}"))?;
    serde_json::to_value(event)
        .map_err(|error| format!("could not encode managed key challenge: {error}"))
}

#[cfg(feature = "evaos-teams-managed")]
fn disable_managed_access(app_state: &AppState) {
    app_state.disable_evaos_teams_access();
}

#[cfg(feature = "evaos-teams-managed")]
fn install_entitlement(
    app_state: &AppState,
    keys: &Keys,
    entitlement: &EvaosTeamsEntitlement,
) -> Result<(), String> {
    let relay = validate_entitlement(entitlement, &keys.public_key().to_hex())?;
    if !entitlement_access_ready(entitlement) {
        return Err("managed relay projection is not current".to_string());
    }
    let expires_at = chrono::DateTime::parse_from_rfc3339(&entitlement.expires_at)
        .map_err(|_| "managed entitlement expiry is invalid".to_string())?
        .timestamp();
    app_state.install_evaos_teams_access(keys.clone(), relay, expires_at)
}

#[cfg(feature = "evaos-teams-managed")]
fn apply_entitlement_status(
    app_state: &AppState,
    keys: &Keys,
    entitlement: EvaosTeamsEntitlement,
) -> Result<EvaosTeamsAuthStatus, String> {
    validate_entitlement(&entitlement, &keys.public_key().to_hex())?;
    if entitlement_access_ready(&entitlement) {
        install_entitlement(app_state, keys, &entitlement)?;
        Ok(EvaosTeamsAuthStatus::active(entitlement))
    } else {
        disable_managed_access(app_state);
        let message = entitlement_sync_message(&entitlement);
        Ok(EvaosTeamsAuthStatus::sync_pending(entitlement, message))
    }
}

#[cfg(feature = "evaos-teams-managed")]
fn initialize_runtime(state: &EvaosTeamsState) -> Result<(), String> {
    let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    if runtime.initialized {
        return Ok(());
    }
    let stored = managed_store().load_all_readonly()?;
    let Some(stored) = stored else {
        runtime.initialized = true;
        return Ok(());
    };
    let identity = stored.get(IDENTITY_KEY);
    let session = stored.get(SESSION_KEY);
    match (identity, session) {
        (None, None) if stored.is_empty() => {
            runtime.initialized = true;
            Ok(())
        }
        (Some(identity), Some(session)) if !session.trim().is_empty() => {
            runtime.keys = Some(
                Keys::parse(identity.trim())
                    .map_err(|_| "managed Keychain identity is invalid".to_string())?,
            );
            runtime.session = Some(Zeroizing::new(session.clone()));
            runtime.logout_pending = stored
                .get(LOGOUT_PENDING_KEY)
                .is_some_and(|value| value == "1");
            runtime.logout_confirmed = stored
                .get(LOGOUT_CONFIRMED_KEY)
                .is_some_and(|value| value == "1");
            if runtime.logout_pending && runtime.logout_confirmed {
                return Err("managed logout state is inconsistent".to_string());
            }
            runtime.previous_session = stored
                .get(PREVIOUS_SESSION_KEY)
                .filter(|value| {
                    (runtime.logout_pending || runtime.logout_confirmed) && !value.trim().is_empty()
                })
                .map(|value| Zeroizing::new(value.clone()));
            runtime.initialized = true;
            Ok(())
        }
        _ => Err("managed Keychain identity and session are incomplete".to_string()),
    }
}

#[cfg(feature = "evaos-teams-managed")]
async fn current_credentials(
    state: &EvaosTeamsState,
) -> Result<
    (
        Zeroizing<String>,
        Keys,
        bool,
        bool,
        Option<Zeroizing<String>>,
    ),
    String,
> {
    initialize_runtime(state)?;
    let runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    let session = runtime
        .session
        .clone()
        .ok_or_else(|| "Sign in to evaOS Teams first".to_string())?;
    let keys = runtime
        .keys
        .clone()
        .ok_or_else(|| "Managed identity is unavailable".to_string())?;
    Ok((
        session,
        keys,
        runtime.logout_pending,
        runtime.logout_confirmed,
        runtime.previous_session.clone(),
    ))
}

#[cfg(feature = "evaos-teams-managed")]
async fn remote_logout(client: &reqwest::Client, token: &str) -> Result<(), ApiFailure> {
    let response: LogoutResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({ "action": "logout" }),
    )
    .await?;
    if response.status == "logged_out" {
        Ok(())
    } else {
        Err(ApiFailure {
            status: reqwest::StatusCode::BAD_GATEWAY,
            code: "invalid_logout_response".to_string(),
        })
    }
}

#[cfg(feature = "evaos-teams-managed")]
fn wipe_managed_store() -> Result<(), String> {
    managed_store().delete_all_with_legacy_cleanup()?;
    if !managed_store().verify_fully_wiped() {
        return Err("could not verify managed Keychain wipe".to_string());
    }
    Ok(())
}

#[cfg(feature = "evaos-teams-managed")]
fn reset_runtime_for_reload(state: &EvaosTeamsState, app_state: &AppState) {
    disable_managed_access(app_state);
    if let Ok(mut runtime) = state.runtime.lock() {
        *runtime = ManagedRuntime::default();
    }
}

#[cfg(feature = "evaos-teams-managed")]
fn stage_pending_claim(
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: &str,
    keys: &Keys,
    previous_session: Option<&str>,
) -> Result<(), String> {
    disable_managed_access(app_state);
    let pending = managed_credential_entries(keys, session, true, false, previous_session)?;
    managed_store()
        .replace_all(&pending)
        .map_err(|_| "Could not record the pending managed session in Keychain".to_string())?;
    if managed_store().load_all_readonly()? != Some(pending) {
        return Err("Managed Keychain pending-session verification failed".to_string());
    }
    let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    *runtime = ManagedRuntime {
        initialized: true,
        session: Some(Zeroizing::new(session.to_string())),
        previous_session: previous_session.map(|value| Zeroizing::new(value.to_string())),
        keys: Some(keys.clone()),
        logout_pending: true,
        logout_confirmed: false,
    };
    Ok(())
}

#[cfg(feature = "evaos-teams-managed")]
async fn finish_failed_staged_login(
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: &str,
    original_error: String,
) -> Result<EvaosTeamsAuthStatus, String> {
    let outcome = retry_pending_logout(&app_state.http_client, state, app_state, session).await;
    if outcome.phase == "signed_out" {
        Err(original_error)
    } else {
        Ok(outcome)
    }
}

#[cfg(feature = "evaos-teams-managed")]
async fn get_remote_entitlement(
    client: &reqwest::Client,
    token: &str,
) -> Result<EvaosTeamsEntitlement, ApiFailure> {
    let response: EntitlementResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({ "action": "get_entitlement" }),
    )
    .await?;
    if response.status == "active" {
        Ok(response.entitlement)
    } else {
        Err(ApiFailure {
            status: reqwest::StatusCode::BAD_GATEWAY,
            code: "inactive_entitlement".to_string(),
        })
    }
}

#[cfg(feature = "evaos-teams-managed")]
fn persist_managed_credentials(
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: String,
    keys: Keys,
    entitlement: EvaosTeamsEntitlement,
) -> Result<EvaosTeamsAuthStatus, String> {
    let replacement = managed_credential_entries(&keys, &session, false, false, None)?;
    let activation = (|| -> Result<EvaosTeamsAuthStatus, String> {
        managed_store()
            .replace_all(&replacement)
            .map_err(|_| "Could not save managed access in macOS Keychain".to_string())?;
        if managed_store().load_all_readonly()? != Some(replacement) {
            return Err("Managed Keychain read-back verification failed".to_string());
        }
        let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
        *runtime = ManagedRuntime {
            initialized: true,
            session: Some(Zeroizing::new(session.clone())),
            previous_session: None,
            keys: Some(keys.clone()),
            logout_pending: false,
            logout_confirmed: false,
        };
        drop(runtime);
        apply_entitlement_status(app_state, &keys, entitlement)
    })();
    match activation {
        Ok(status) => Ok(status),
        Err(error) => {
            reset_runtime_for_reload(state, app_state);
            Err(error)
        }
    }
}

/// Return current managed-auth status and perform a bounded entitlement refresh.
#[tauri::command]
pub(crate) async fn get_evaos_teams_auth_status(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Ok(EvaosTeamsAuthStatus::unmanaged())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        if let Err(error) = initialize_runtime(&state) {
            disable_managed_access(&app_state);
            return Ok(EvaosTeamsAuthStatus::locked(error));
        }
        let credentials = current_credentials(&state).await;
        let (session, keys, logout_pending, logout_confirmed, _) = match credentials {
            Ok(value) => value,
            Err(_) => {
                disable_managed_access(&app_state);
                return Ok(EvaosTeamsAuthStatus::signed_out());
            }
        };
        if logout_pending || logout_confirmed {
            return Ok(
                retry_pending_logout(&app_state.http_client, &state, &app_state, &session).await,
            );
        }
        let response = get_remote_entitlement(&app_state.http_client, &session).await;
        match response {
            Ok(entitlement) => match apply_entitlement_status(&app_state, &keys, entitlement) {
                Ok(status) => Ok(status),
                Err(error) => {
                    disable_managed_access(&app_state);
                    Ok(EvaosTeamsAuthStatus::reauth(error))
                }
            },
            Err(error) => {
                disable_managed_access(&app_state);
                Ok(EvaosTeamsAuthStatus::reauth(format!(
                    "Managed access could not be refreshed: {error}"
                )))
            }
        }
    }
}

#[cfg(feature = "evaos-teams-managed")]
async fn claim_device_code(
    client: &reqwest::Client,
    device_code: &str,
) -> Result<ClaimResponse, String> {
    let response: ClaimResponse = post_json(
        client,
        "desktop-runtime-session",
        None,
        serde_json::json!({
            "action": "claim_desktop_device_code",
            "device_code": device_code,
        }),
    )
    .await
    .map_err(|error| format!("Device sign-in could not be claimed: {error}"))?;
    if response.desktop_session.len() < 32 || response.desktop_session.len() > 512 {
        return Err("Device sign-in returned an invalid Desktop session".to_string());
    }
    let expires_at = chrono::DateTime::parse_from_rfc3339(&response.desktop_session_expires_at)
        .map_err(|_| "Device sign-in returned an invalid expiry".to_string())?;
    if expires_at <= chrono::Utc::now() {
        return Err("Device sign-in returned an expired Desktop session".to_string());
    }
    Ok(response)
}

#[cfg(feature = "evaos-teams-managed")]
async fn bind_identity(
    client: &reqwest::Client,
    token: &str,
    keys: &Keys,
) -> Result<EvaosTeamsEntitlement, String> {
    let public_key = keys.public_key().to_hex();
    let challenge: ChallengeResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "issue_key_challenge",
            "public_key": public_key,
            "device_metadata": {
                "label": "evaOS Teams",
                "app_version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS,
            },
        }),
    )
    .await
    .map_err(|error| format!("Managed key challenge was not available: {error}"))?;
    let signed_event = signed_challenge(&challenge, keys)?;
    let verified: EntitlementResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({
            "action": "verify_key_challenge",
            "signed_event": signed_event,
        }),
    )
    .await
    .map_err(|error| format!("Managed key challenge was rejected: {error}"))?;
    if verified.status != "active" {
        return Err("Managed key verification did not activate access".to_string());
    }
    // The verification response intentionally omits public_key; the key is
    // already bound by the exact signed challenge above. Install the locally
    // derived public key so every later entitlement validation stays strict.
    bind_verified_entitlement(verified.entitlement, &challenge.relay_host, &public_key)
}

/// Start an account-selecting browser login and complete device-code claim and
/// server key binding entirely in Rust.
#[tauri::command]
pub(crate) async fn start_evaos_teams_login(
    app: tauri::AppHandle,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&app, &state, &app_state);
        Err("evaOS Teams managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        // Prove Keychain reachability before opening the browser. Managed mode
        // never falls back to a plaintext identity or Desktop token.
        verify_managed_store_writable()?;

        initialize_runtime(&state)?;
        let mut reauth_credentials = None;
        if let Ok((session, keys, logout_pending, logout_confirmed, _)) =
            current_credentials(&state).await
        {
            if logout_pending || logout_confirmed {
                return Ok(retry_pending_logout(
                    &app_state.http_client,
                    &state,
                    &app_state,
                    &session,
                )
                .await);
            } else {
                match get_remote_entitlement(&app_state.http_client, &session).await {
                    Ok(entitlement)
                        if validate_entitlement(&entitlement, &keys.public_key().to_hex())
                            .is_ok() =>
                    {
                        // An explicit account switch starts with a confirmed,
                        // atomic logout. If browser sign-in is later canceled,
                        // the previous account remains safely revoked.
                        let outcome = begin_managed_logout(&state, &app_state).await?;
                        if outcome.phase != "signed_out" {
                            return Ok(outcome);
                        }
                    }
                    _ => {
                        // The old Desktop token can no longer prove logout.
                        // Permit only same-account reauthentication, established
                        // after claim by an entitlement for this exact old key.
                        reauth_credentials = Some((session, keys));
                    }
                }
            }
        }

        disable_managed_access(&app_state);
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| format!("could not start local sign-in callback: {error}"))?;
        let port = listener
            .local_addr()
            .map_err(|error| format!("could not read local sign-in callback: {error}"))?
            .port();
        let auth_state = uuid::Uuid::new_v4().simple().to_string();
        let device_code = uuid::Uuid::new_v4().simple().to_string().to_uppercase();
        let callback = format!("http://127.0.0.1:{port}/auth/callback");
        let login_url = dashboard_login_url(&callback, &auth_state, &device_code)?;
        let (sender, receiver) = oneshot::channel();
        let callback_state = Arc::new(LoginCallback::new(auth_state, device_code, sender));
        *state
            .pending_login
            .lock()
            .map_err(|error| error.to_string())? = Some(callback_state.clone());
        let router = Router::new()
            .route("/auth/callback", get(login_callback))
            .with_state(callback_state.clone());
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        if let Err(error) = app.opener().open_url(login_url.as_str(), None::<&str>) {
            server.abort();
            clear_pending_login(&state, &callback_state);
            return Err(format!("could not open ElectricSheep sign-in: {error}"));
        }
        let code = match tokio::time::timeout(LOGIN_TIMEOUT, receiver).await {
            Ok(Ok(Ok(code))) => code,
            Ok(Ok(Err(error))) => {
                server.abort();
                clear_pending_login(&state, &callback_state);
                return Err(error);
            }
            Ok(Err(_)) => {
                server.abort();
                clear_pending_login(&state, &callback_state);
                return Err("local sign-in callback stopped unexpectedly".to_string());
            }
            Err(_) => {
                server.abort();
                clear_pending_login(&state, &callback_state);
                return Err("ElectricSheep sign-in timed out".to_string());
            }
        };
        server.abort();
        clear_pending_login(&state, &callback_state);

        let claim = claim_device_code(&app_state.http_client, &code).await?;
        let keys = reauth_credentials
            .as_ref()
            .map(|(_, keys)| keys.clone())
            .unwrap_or_else(Keys::generate);
        let previous_session = reauth_credentials
            .as_ref()
            .map(|(session, _)| session.as_str());
        if let Err(error) = stage_pending_claim(
            &state,
            &app_state,
            &claim.desktop_session,
            &keys,
            previous_session,
        ) {
            // The dashboard claim response is the first point at which the
            // bearer exists, so a crash before this write cannot be recovered
            // client-side. On an observed write failure, synchronously revoke
            // the new session and report whether that cleanup was confirmed.
            return match remote_logout(&app_state.http_client, &claim.desktop_session).await {
                Ok(()) => {
                    reset_runtime_for_reload(&state, &app_state);
                    Err(error)
                }
                Err(cleanup_error) => Err(format!(
                    "{error}; the newly claimed session could not be durably recorded or confirmed revoked: {cleanup_error}"
                )),
            };
        }

        if reauth_credentials.is_some() {
            match get_remote_entitlement(&app_state.http_client, &claim.desktop_session).await {
                Ok(entitlement)
                    if validate_entitlement(&entitlement, &keys.public_key().to_hex()).is_ok() =>
                {
                    return persist_managed_credentials(
                        &state,
                        &app_state,
                        claim.desktop_session,
                        keys,
                        entitlement,
                    );
                }
                _ => {
                    return finish_failed_staged_login(
                        &state,
                        &app_state,
                        &claim.desktop_session,
                        "The previous account can no longer prove logout, so evaOS Teams will not replace it with a different account"
                            .to_string(),
                    )
                    .await;
                }
            }
        }
        let entitlement =
            match bind_identity(&app_state.http_client, &claim.desktop_session, &keys).await {
                Ok(entitlement) => entitlement,
                Err(error) => {
                    return finish_failed_staged_login(
                        &state,
                        &app_state,
                        &claim.desktop_session,
                        error,
                    )
                    .await;
                }
            };
        persist_managed_credentials(&state, &app_state, claim.desktop_session, keys, entitlement)
    }
}

/// Revoke the server session and active human relay identity, then wipe only
/// the managed Keychain service. A network-ambiguous logout remains durable
/// and is retried on the next status check or relaunch.
#[tauri::command]
pub(crate) async fn logout_evaos_teams(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Err("evaOS Teams managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        begin_managed_logout(&state, &app_state).await
    }
}

#[cfg(test)]
mod tests;
