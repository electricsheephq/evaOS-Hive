//! Electric-only managed admission for Hive.
//!
//! This adapter deliberately reuses Buzz's resolved native identity and native
//! workspace/runtime paths. Electric stores only an opaque desktop session; it
//! never receives or persists the native private key.
#![cfg_attr(not(feature = "evaos-teams-managed"), allow(dead_code, unused_imports))]

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::Duration,
};

use axum::{
    extract::{Query, State as AxumState},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use nostr::{EventBuilder, Keys, Kind, Tag, Timestamp};
use serde::{Deserialize, Serialize};
use tauri::State;
use tauri_plugin_opener::OpenerExt;
use tokio::{net::TcpListener, sync::oneshot};
use url::Url;
use zeroize::Zeroizing;

use crate::{app_state::AppState, secret_store::SecretStore};
#[cfg(test)]
use device_code::device_code_challenge;
use device_code::{dashboard_login_url, normalize_device_code, DeviceCodeProof};

mod company_directory;
mod device_code;

pub(crate) use company_directory::list_hive_company_agents;

const DASHBOARD_ORIGIN: &str = "https://www.electricsheephq.com";
const SUPABASE_ORIGIN: &str = "https://rhfojelkgtwcxnrfhtlj.supabase.co";
// The publishable client identifier is injected only into managed builds. It
// is intentionally absent from source control and is not an authorization
// credential; authorization still comes only from the opaque desktop session.
const SUPABASE_PUBLISHABLE_KEY: Option<&str> = option_env!("HIVE_SUPABASE_PUBLISHABLE_KEY");
const KEYRING_SERVICE: &str = "evaos-teams-desktop";
const SESSION_KEY: &str = "electric_desktop_session";
const LOGOUT_PENDING_KEY: &str = "logout_pending";
const KEY_BINDING_KIND: u16 = 27_235;
const KEY_BINDING_SCHEMA: &str = "evaos.buzz_key_binding.v1";
const LOGIN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

fn managed_store() -> &'static SecretStore {
    static STORE: OnceLock<SecretStore> = OnceLock::new();
    STORE.get_or_init(|| SecretStore::keyring(KEYRING_SERVICE))
}

#[cfg(feature = "evaos-teams-managed")]
fn verify_managed_store_writable() -> Result<(), String> {
    let existing = managed_store()
        .load_all_readonly()
        .map_err(|_| "Unlock macOS Keychain, then try again".to_string())?
        .unwrap_or_default();
    managed_store()
        .replace_all(&existing)
        .map_err(|_| "Hive cannot write to macOS Keychain".to_string())?;
    if managed_store().load_all_readonly()? != Some(existing) {
        return Err("Hive could not verify its Keychain write".to_string());
    }
    Ok(())
}

/// Renderer-safe projection of a server-selected managed entitlement.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "snake_case"))]
pub(crate) struct EvaosTeamsEntitlement {
    community_id: String,
    relay_host: String,
    public_key: Option<String>,
    role: String,
    access_revision: u64,
    expires_at: String,
    refresh_after_seconds: u64,
}

/// Public authentication state. Session, verifier, challenge, and private key
/// material are deliberately absent.
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

    fn managed(phase: &'static str, message: Option<String>) -> Self {
        Self {
            managed: true,
            phase,
            authenticated: false,
            keychain_available: phase != "keychain_locked",
            message,
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

    fn identity_restore_required(message: impl Into<String>) -> Self {
        Self::managed("identity_restore_required", Some(message.into()))
    }
}

#[derive(Default)]
struct ManagedRuntime {
    initialized: bool,
    session: Option<Zeroizing<String>>,
    logout_pending: bool,
}

/// Backend-only managed session state.
#[derive(Default)]
pub(crate) struct EvaosTeamsState {
    runtime: Mutex<ManagedRuntime>,
    operation: tokio::sync::Mutex<()>,
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

#[derive(Debug, Deserialize, PartialEq)]
struct IdentityBinding {
    membership_id: String,
    public_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdentityBindingResponse {
    status: String,
    binding: IdentityBinding,
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

#[derive(Debug)]
struct ApiFailure {
    status: reqwest::StatusCode,
    code: String,
}

impl std::fmt::Display for ApiFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "HTTP {} ({})", self.status, self.code)
    }
}

impl ApiFailure {
    fn means_session_is_absent(&self) -> bool {
        matches!(self.status.as_u16(), 401 | 404)
    }
}

fn functions_url(name: &str) -> Result<Url, String> {
    Url::parse(&format!("{SUPABASE_ORIGIN}/functions/v1/{name}"))
        .map_err(|error| format!("invalid managed API URL: {error}"))
}

async fn post_json<T: for<'de> Deserialize<'de>>(
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
        return Err(ApiFailure {
            status,
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

fn relay_websocket_url(relay_host: &str) -> Result<String, String> {
    let mut url = Url::parse(relay_host)
        .map_err(|_| "managed entitlement has an invalid relay origin".to_string())?;
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

fn valid_public_key(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_entitlement(
    entitlement: &EvaosTeamsEntitlement,
    expected_public_key: &str,
) -> Result<String, String> {
    uuid::Uuid::parse_str(&entitlement.community_id)
        .map_err(|_| "managed entitlement has an invalid community".to_string())?;
    if !valid_public_key(expected_public_key)
        || entitlement.public_key.as_deref() != Some(expected_public_key)
    {
        return Err("managed entitlement belongs to a different identity".to_string());
    }
    if entitlement.role.trim().is_empty()
        || entitlement.refresh_after_seconds < 30
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

fn bind_verified_entitlement(
    mut entitlement: EvaosTeamsEntitlement,
    challenge: &ChallengeResponse,
    public_key: &str,
) -> Result<EvaosTeamsEntitlement, String> {
    if entitlement.relay_host != challenge.relay_host
        || entitlement.community_id != challenge.challenge.community_id
        || entitlement.public_key.is_some()
    {
        return Err("managed key verification changed the server-selected scope".to_string());
    }
    entitlement.public_key = Some(public_key.to_string());
    validate_entitlement(&entitlement, public_key)?;
    Ok(entitlement)
}

fn validate_challenge(
    response: &ChallengeResponse,
    expected_public_key: &str,
    expected_membership_id: &str,
) -> Result<(), String> {
    if response.status != "challenge_issued"
        || response.challenge.schema_version != KEY_BINDING_SCHEMA
        || response.challenge.public_key != expected_public_key
        || response.challenge.membership_id != expected_membership_id
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
    expected_membership_id: &str,
) -> Result<serde_json::Value, String> {
    validate_challenge(
        response,
        &keys.public_key().to_hex(),
        expected_membership_id,
    )?;
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

fn verify_existing_native_identity(binding: &IdentityBinding, keys: &Keys) -> Result<(), String> {
    uuid::Uuid::parse_str(&binding.membership_id)
        .map_err(|_| "Electric Sheep returned an invalid membership".to_string())?;
    let canonical = binding
        .public_key
        .as_deref()
        .filter(|value| valid_public_key(value))
        .ok_or_else(|| {
            "Electric Sheep has no canonical Hive identity for this membership".to_string()
        })?;
    if canonical != keys.public_key().to_hex() {
        return Err(
            "This device's native Buzz identity does not match the canonical Hive identity"
                .to_string(),
        );
    }
    Ok(())
}

fn disable_managed_access(app_state: &AppState) {
    app_state
        .evaos_teams_authorized
        .store(false, std::sync::atomic::Ordering::Release);
    if let Ok(mut relay) = app_state.relay_url_override.lock() {
        *relay = None;
    }
}

fn install_entitlement(
    app_state: &AppState,
    keys: &Keys,
    entitlement: &EvaosTeamsEntitlement,
) -> Result<(), String> {
    let relay = validate_entitlement(entitlement, &keys.public_key().to_hex())?;
    disable_managed_access(app_state);
    *app_state
        .relay_url_override
        .lock()
        .map_err(|error| error.to_string())? = Some(relay);
    app_state
        .managed_agent_restore_pending
        .store(true, std::sync::atomic::Ordering::Release);
    app_state
        .evaos_teams_authorized
        .store(true, std::sync::atomic::Ordering::Release);
    Ok(())
}

fn runtime_from_entries(stored: Option<HashMap<String, String>>) -> Result<ManagedRuntime, String> {
    let stored = stored.unwrap_or_default();
    if stored
        .keys()
        .any(|key| key != SESSION_KEY && key != LOGOUT_PENDING_KEY)
    {
        return Err("managed Keychain contains unsupported credential material".to_string());
    }
    let session = stored
        .get(SESSION_KEY)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .map(Zeroizing::new);
    let logout_pending = stored
        .get(LOGOUT_PENDING_KEY)
        .is_some_and(|value| value == "1");
    if logout_pending && session.is_none() {
        return Err("managed logout marker has no Desktop session".to_string());
    }
    Ok(ManagedRuntime {
        initialized: true,
        session,
        logout_pending,
    })
}

#[cfg(feature = "evaos-teams-managed")]
fn initialize_runtime(state: &EvaosTeamsState) -> Result<(), String> {
    let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    if runtime.initialized {
        return Ok(());
    }
    *runtime = runtime_from_entries(managed_store().load_all_readonly()?)?;
    Ok(())
}

#[cfg(feature = "evaos-teams-managed")]
fn current_session(state: &EvaosTeamsState) -> Result<(Zeroizing<String>, bool), String> {
    initialize_runtime(state)?;
    let runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    let session = runtime
        .session
        .clone()
        .ok_or_else(|| "Sign in to Hive first".to_string())?;
    Ok((session, runtime.logout_pending))
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
fn persist_signed_out(state: &EvaosTeamsState) -> Result<(), String> {
    let replacement = HashMap::new();
    managed_store().replace_all(&replacement)?;
    if managed_store().load_all_readonly()? != Some(replacement) {
        return Err("managed session read-back verification failed".to_string());
    }
    *state.runtime.lock().map_err(|error| error.to_string())? = ManagedRuntime {
        initialized: true,
        ..ManagedRuntime::default()
    };
    Ok(())
}

#[cfg(feature = "evaos-teams-managed")]
async fn retry_pending_logout(
    client: &reqwest::Client,
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: &str,
) -> EvaosTeamsAuthStatus {
    disable_managed_access(app_state);
    if let Err(error) = remote_logout(client, session).await {
        if !error.means_session_is_absent() {
            return EvaosTeamsAuthStatus::managed(
                "logout_pending",
                Some(format!("Remote logout is still pending: {error}")),
            );
        }
    }
    match persist_signed_out(state) {
        Ok(()) => EvaosTeamsAuthStatus::managed("signed_out", None),
        Err(error) => EvaosTeamsAuthStatus::managed("keychain_locked", Some(error)),
    }
}

#[cfg(feature = "evaos-teams-managed")]
async fn begin_managed_logout(
    state: &EvaosTeamsState,
    app_state: &AppState,
) -> Result<EvaosTeamsAuthStatus, String> {
    let (session, _) = current_session(state)?;
    disable_managed_access(app_state);
    let pending = HashMap::from([
        (SESSION_KEY.to_string(), session.to_string()),
        (LOGOUT_PENDING_KEY.to_string(), "1".to_string()),
    ]);
    managed_store()
        .replace_all(&pending)
        .map_err(|_| "Could not record durable managed logout".to_string())?;
    if managed_store().load_all_readonly()? != Some(pending) {
        return Err("Could not verify durable managed logout".to_string());
    }
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.logout_pending = true;
    }
    Ok(retry_pending_logout(&app_state.http_client, state, app_state, &session).await)
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
async fn get_identity_binding(
    client: &reqwest::Client,
    token: &str,
) -> Result<IdentityBinding, String> {
    let response: IdentityBindingResponse = post_json(
        client,
        "evaos-teams-access",
        Some(token),
        serde_json::json!({ "action": "get_identity_binding" }),
    )
    .await
    .map_err(|error| format!("Managed identity selection was not available: {error}"))?;
    if response.status != "active" {
        return Err("Managed identity selection is not active".to_string());
    }
    uuid::Uuid::parse_str(&response.binding.membership_id)
        .map_err(|_| "Managed identity selection returned an invalid membership".to_string())?;
    Ok(response.binding)
}

#[cfg(feature = "evaos-teams-managed")]
async fn bind_identity(
    client: &reqwest::Client,
    token: &str,
    keys: &Keys,
    expected_membership_id: &str,
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
                "label": "Hive",
                "app_version": env!("CARGO_PKG_VERSION"),
                "platform": std::env::consts::OS,
            },
        }),
    )
    .await
    .map_err(|error| format!("Managed key challenge was not available: {error}"))?;
    let signed_event = signed_challenge(&challenge, keys, expected_membership_id)?;
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
    bind_verified_entitlement(verified.entitlement, &challenge, &public_key)
}

#[cfg(feature = "evaos-teams-managed")]
fn persist_active_session(
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: String,
    keys: &Keys,
    entitlement: EvaosTeamsEntitlement,
) -> Result<EvaosTeamsAuthStatus, String> {
    let replacement = HashMap::from([(SESSION_KEY.to_string(), session.clone())]);
    managed_store()
        .replace_all(&replacement)
        .map_err(|_| "Could not save managed access in macOS Keychain".to_string())?;
    if managed_store().load_all_readonly()? != Some(replacement) {
        return Err("Managed Keychain read-back verification failed".to_string());
    }
    install_entitlement(app_state, keys, &entitlement)?;
    *state.runtime.lock().map_err(|error| error.to_string())? = ManagedRuntime {
        initialized: true,
        session: Some(Zeroizing::new(session)),
        logout_pending: false,
    };
    Ok(EvaosTeamsAuthStatus::active(entitlement))
}

/// Return current managed-auth status and perform one bounded entitlement
/// refresh. Unmanaged builds retain the native bypass.
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
            return Ok(EvaosTeamsAuthStatus::managed(
                "keychain_locked",
                Some(error),
            ));
        }
        let (session, logout_pending) = match current_session(&state) {
            Ok(value) => value,
            Err(_) => {
                disable_managed_access(&app_state);
                return Ok(EvaosTeamsAuthStatus::managed("signed_out", None));
            }
        };
        if logout_pending {
            return Ok(
                retry_pending_logout(&app_state.http_client, &state, &app_state, &session).await,
            );
        }
        let keys = match app_state.native_identity_for_managed_verification() {
            Ok(keys) => keys,
            Err(error) => {
                disable_managed_access(&app_state);
                return Ok(EvaosTeamsAuthStatus::identity_restore_required(error));
            }
        };
        let binding = match get_identity_binding(&app_state.http_client, &session).await {
            Ok(binding) => binding,
            Err(error) => {
                disable_managed_access(&app_state);
                return Ok(EvaosTeamsAuthStatus::managed(
                    "reauth_required",
                    Some(error),
                ));
            }
        };
        if let Err(error) = verify_existing_native_identity(&binding, &keys) {
            disable_managed_access(&app_state);
            return Ok(EvaosTeamsAuthStatus::identity_restore_required(error));
        }
        match get_remote_entitlement(&app_state.http_client, &session).await {
            Ok(entitlement) => match install_entitlement(&app_state, &keys, &entitlement) {
                Ok(()) => Ok(EvaosTeamsAuthStatus::active(entitlement)),
                Err(error) => {
                    disable_managed_access(&app_state);
                    Ok(EvaosTeamsAuthStatus::managed(
                        "reauth_required",
                        Some(error),
                    ))
                }
            },
            Err(error) => {
                disable_managed_access(&app_state);
                Ok(EvaosTeamsAuthStatus::managed(
                    "reauth_required",
                    Some(format!("Managed access could not be refreshed: {error}")),
                ))
            }
        }
    }
}

struct LoginCallback {
    expected_state: String,
    sender: Mutex<Option<oneshot::Sender<Result<String, String>>>>,
}

fn callback_device_code(
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

async fn login_callback(
    Query(query): Query<HashMap<String, String>>,
    AxumState(state): AxumState<std::sync::Arc<LoginCallback>>,
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

#[cfg(feature = "evaos-teams-managed")]
async fn claim_device_code(
    client: &reqwest::Client,
    device_code: &str,
    device_code_verifier: &str,
) -> Result<ClaimResponse, String> {
    let response: ClaimResponse = post_json(
        client,
        "desktop-runtime-session",
        None,
        serde_json::json!({
            "action": "claim_desktop_device_code",
            "device_code": device_code,
            "device_code_verifier": device_code_verifier,
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

/// Start proof-bound browser OAuth, verify the already-resolved native identity,
/// and install only the server-selected entitlement.
#[tauri::command]
pub(crate) async fn start_evaos_teams_login(
    app: tauri::AppHandle,
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&app, &state, &app_state);
        Err("Hive managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        verify_managed_store_writable()?;
        initialize_runtime(&state)?;
        disable_managed_access(&app_state);

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| format!("could not start local sign-in callback: {error}"))?;
        let port = listener
            .local_addr()
            .map_err(|error| format!("could not read local sign-in callback: {error}"))?
            .port();
        let auth_state = uuid::Uuid::new_v4().simple().to_string();
        let device_code_proof = DeviceCodeProof::new();
        let callback = format!("http://127.0.0.1:{port}/auth/callback");
        let login_url = dashboard_login_url(&callback, &auth_state, &device_code_proof.challenge)?;
        let (sender, receiver) = oneshot::channel();
        let callback_state = std::sync::Arc::new(LoginCallback {
            expected_state: auth_state,
            sender: Mutex::new(Some(sender)),
        });
        let router = Router::new()
            .route("/auth/callback", get(login_callback))
            .with_state(callback_state);
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });

        if let Err(error) = app.opener().open_url(login_url.as_str(), None::<&str>) {
            server.abort();
            return Err(format!("could not open Electric Sheep sign-in: {error}"));
        }
        let code = match tokio::time::timeout(LOGIN_TIMEOUT, receiver).await {
            Ok(Ok(Ok(code))) => code,
            Ok(Ok(Err(error))) => {
                server.abort();
                return Err(error);
            }
            Ok(Err(_)) => {
                server.abort();
                return Err("local sign-in callback stopped unexpectedly".to_string());
            }
            Err(_) => {
                server.abort();
                return Err("Electric Sheep sign-in timed out".to_string());
            }
        };
        server.abort();

        let claim =
            claim_device_code(&app_state.http_client, &code, &device_code_proof.verifier).await?;
        let keys = match app_state.native_identity_for_managed_verification() {
            Ok(keys) => keys,
            Err(error) => return Ok(EvaosTeamsAuthStatus::identity_restore_required(error)),
        };
        let binding = get_identity_binding(&app_state.http_client, &claim.desktop_session).await?;
        if let Err(error) = verify_existing_native_identity(&binding, &keys) {
            return Ok(EvaosTeamsAuthStatus::identity_restore_required(error));
        }
        let entitlement = bind_identity(
            &app_state.http_client,
            &claim.desktop_session,
            &keys,
            &binding.membership_id,
        )
        .await?;
        let rollback_session = claim.desktop_session.clone();
        let result = persist_active_session(
            &state,
            &app_state,
            claim.desktop_session,
            &keys,
            entitlement,
        );
        if result.is_err() {
            let _ = remote_logout(&app_state.http_client, &rollback_session).await;
        }
        result
    }
}

/// Revoke only the opaque Electric device session. The native Buzz identity
/// remains untouched so offline messages keep their durable recipient.
#[tauri::command]
pub(crate) async fn logout_evaos_teams(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<EvaosTeamsAuthStatus, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Err("Hive managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        begin_managed_logout(&state, &app_state).await
    }
}

#[cfg(test)]
mod tests;
