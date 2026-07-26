//! Electric-only evaOS Teams authentication.
//!
//! This module is intentionally separate from Buzz's native identity path.
//! Managed builds use an opaque Electric Desktop session plus a Nostr key held
//! under a distinct OS-keyring service. Neither value is serialized to the
//! renderer, migrated from native Buzz, or accepted from environment input.
#![cfg_attr(not(feature = "evaos-teams-managed"), allow(dead_code, unused_imports))]

use std::{
    collections::{HashMap, HashSet},
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
use nostr::{EventBuilder, Keys, Kind, Tag, Timestamp, ToBech32};
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;
use tokio::{net::TcpListener, sync::oneshot};
use url::Url;
use zeroize::Zeroizing;

use crate::{app_state::AppState, secret_store::SecretStore};
#[cfg(test)]
use device_code::device_code_challenge;
use device_code::{dashboard_login_url, normalize_device_code, DeviceCodeProof};

mod device_code;

const DASHBOARD_ORIGIN: &str = "https://www.electricsheephq.com";
const SUPABASE_ORIGIN: &str = "https://rhfojelkgtwcxnrfhtlj.supabase.co";
// Supabase publishable keys are intentionally public client identifiers. This
// value grants no service-role access; authorization still comes exclusively
// from the opaque Desktop session returned after browser authentication.
const SUPABASE_PUBLISHABLE_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmUiLCJyZWYiOiJyaGZvamVsa2d0d2N4bnJmaHRsaiIsInJvbGUiOiJhbm9uIiwiaWF0IjoxNzczMjQzNTc2LCJleHAiOjIwODg4MTk1NzZ9.X8mJHaYIolCmx6j_473GGb05OyFTy43Hq-BEelZRjAE";
const KEYRING_SERVICE: &str = "evaos-teams-desktop";
// Kept only to migrate the single-identity layout shipped by the first managed
// candidate. New keys are scoped by the server-selected membership UUID.
const IDENTITY_KEY: &str = "identity";
const IDENTITY_KEY_PREFIX: &str = "identity:";
const ACTIVE_MEMBERSHIP_KEY: &str = "active_membership_id";
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

/// Public, renderer-safe entitlement projection.
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
}

#[derive(Default)]
struct ManagedRuntime {
    initialized: bool,
    session: Option<Zeroizing<String>>,
    keys: Option<Keys>,
    membership_id: Option<String>,
    logout_pending: bool,
}

/// Backend-only managed credential state.
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HiveCompanyAgent {
    agent_instance_id: String,
    public_key: String,
    display_name: String,
    runtime: String,
}

#[derive(Debug, Deserialize)]
struct RawHiveCompanyAgent {
    agent_instance_id: String,
    public_key: String,
    display_name: String,
    runtime: String,
}

#[derive(Debug, Deserialize)]
struct CollaborationProjection {
    #[serde(default)]
    agents: Vec<RawHiveCompanyAgent>,
}

#[derive(Debug, Deserialize)]
struct CollaborationResponse {
    status: String,
    collaboration: CollaborationProjection,
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

fn sanitize_company_agents(agents: Vec<RawHiveCompanyAgent>) -> Vec<HiveCompanyAgent> {
    let mut seen = HashSet::new();
    agents
        .into_iter()
        .filter_map(|agent| {
            let display_name = agent.display_name.trim();
            let runtime = agent.runtime.trim();
            let valid_public_key = agent.public_key.len() == 64
                && agent.public_key.chars().all(|character| {
                    character.is_ascii_hexdigit() && !character.is_ascii_uppercase()
                });
            let valid_text = |value: &str, max: usize| {
                !value.is_empty()
                    && value.len() <= max
                    && value.chars().all(|character| !character.is_control())
            };
            if uuid::Uuid::parse_str(&agent.agent_instance_id).is_err()
                || !valid_public_key
                || !seen.insert(agent.public_key.clone())
                || !valid_text(display_name, 128)
                || !valid_text(runtime, 64)
            {
                return None;
            }
            Some(HiveCompanyAgent {
                agent_instance_id: agent.agent_instance_id,
                public_key: agent.public_key,
                display_name: display_name.to_string(),
                runtime: runtime.to_string(),
            })
        })
        .collect()
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

fn disable_managed_access(app_state: &AppState) {
    app_state
        .evaos_teams_authorized
        .store(false, std::sync::atomic::Ordering::Release);
    if let Ok(mut relay) = app_state.relay_url_override.lock() {
        *relay = None;
    }
}

fn encode_managed_identity(keys: &Keys) -> Result<String, String> {
    keys.secret_key()
        .to_bech32()
        .map_err(|error| format!("could not encode managed identity: {error}"))
}

#[cfg(test)]
fn identity_only_entries(keys: &Keys) -> Result<HashMap<String, String>, String> {
    Ok(HashMap::from([(
        IDENTITY_KEY.to_string(),
        encode_managed_identity(keys)?,
    )]))
}

fn membership_identity_key(membership_id: &str) -> Result<String, String> {
    uuid::Uuid::parse_str(membership_id)
        .map_err(|_| "managed membership identity is invalid".to_string())?;
    Ok(format!("{IDENTITY_KEY_PREFIX}{membership_id}"))
}

fn parse_stored_identity(value: &str) -> Result<Keys, String> {
    Keys::parse(value.trim()).map_err(|_| "managed Keychain identity is invalid".to_string())
}

fn select_login_keys(
    stored: &HashMap<String, String>,
    binding: &IdentityBinding,
) -> Result<Keys, String> {
    let scoped_key = membership_identity_key(&binding.membership_id)?;
    let scoped = stored
        .get(&scoped_key)
        .map(|value| parse_stored_identity(value))
        .transpose()?;

    match binding.public_key.as_deref() {
        Some(public_key) => {
            if public_key.len() != 64
                || !public_key
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err("server returned an invalid managed public identity".to_string());
            }
            if let Some(keys) = scoped {
                if keys.public_key().to_hex() == public_key {
                    return Ok(keys);
                }
            }
            if let Some(value) = stored.get(IDENTITY_KEY) {
                let legacy = parse_stored_identity(value)?;
                if legacy.public_key().to_hex() == public_key {
                    return Ok(legacy);
                }
            }
            Err("This device does not hold the private key for this Hive membership".to_string())
        }
        None => Ok(scoped.unwrap_or_else(Keys::generate)),
    }
}

fn managed_credential_entries(
    mut stored: HashMap<String, String>,
    membership_id: &str,
    keys: &Keys,
    session: &str,
) -> Result<HashMap<String, String>, String> {
    let public_key = keys.public_key();
    let migrated_legacy = stored
        .get(IDENTITY_KEY)
        .map(|value| parse_stored_identity(value))
        .transpose()?
        .is_some_and(|legacy| legacy.public_key() == public_key);
    if migrated_legacy {
        stored.remove(IDENTITY_KEY);
    }
    stored.remove(LOGOUT_PENDING_KEY);
    stored.insert(
        membership_identity_key(membership_id)?,
        encode_managed_identity(keys)?,
    );
    stored.insert(ACTIVE_MEMBERSHIP_KEY.to_string(), membership_id.to_string());
    stored.insert(SESSION_KEY.to_string(), session.to_string());
    Ok(stored)
}

fn runtime_from_entries(stored: Option<HashMap<String, String>>) -> Result<ManagedRuntime, String> {
    let Some(stored) = stored else {
        return Ok(ManagedRuntime {
            initialized: true,
            ..ManagedRuntime::default()
        });
    };
    if stored.is_empty() {
        return Ok(ManagedRuntime {
            initialized: true,
            ..ManagedRuntime::default()
        });
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
    let membership_id = stored.get(ACTIVE_MEMBERSHIP_KEY).cloned();
    let identity = if let Some(membership_id) = membership_id.as_deref() {
        stored.get(&membership_identity_key(membership_id)?)
    } else {
        stored.get(IDENTITY_KEY)
    };
    let keys = identity
        .map(|value| parse_stored_identity(value))
        .transpose()?;
    if session.is_some() && keys.is_none() {
        return Err("managed Keychain identity is missing".to_string());
    }

    Ok(ManagedRuntime {
        initialized: true,
        session,
        keys,
        membership_id,
        logout_pending,
    })
}

fn install_entitlement(
    app_state: &AppState,
    keys: &Keys,
    entitlement: &EvaosTeamsEntitlement,
) -> Result<(), String> {
    let relay = validate_entitlement(entitlement, &keys.public_key().to_hex())?;
    disable_managed_access(app_state);
    *app_state.keys.lock().map_err(|error| error.to_string())? = keys.clone();
    *app_state
        .relay_url_override
        .lock()
        .map_err(|error| error.to_string())? = Some(relay);
    app_state
        .evaos_teams_authorized
        .store(true, std::sync::atomic::Ordering::Release);
    app_state
        .managed_agent_restore_pending
        .store(true, std::sync::atomic::Ordering::Release);
    start_authenticated_native_runtime(app_state, keys);
    Ok(())
}

fn start_authenticated_native_runtime(app_state: &AppState, keys: &Keys) {
    if app_state
        .evaos_teams_native_runtime_started
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
        )
        .is_err()
    {
        return;
    }
    let app = app_state
        .app_handle
        .lock()
        .ok()
        .and_then(|handle| handle.clone());
    let Some(app) = app else {
        app_state
            .evaos_teams_native_runtime_started
            .store(false, std::sync::atomic::Ordering::Release);
        return;
    };

    crate::event_sync::spawn_event_sync(app.clone(), keys.clone());
    tauri::async_runtime::spawn(async move {
        use std::time::Duration;

        let Ok(db_path) = crate::managed_agents::managed_agents_base_dir(&app)
            .map(|directory| directory.join("retention.db"))
        else {
            eprintln!("buzz-desktop: managed event-flush cannot resolve retention db path");
            return;
        };
        loop {
            let state = app.state::<AppState>();
            if state
                .evaos_teams_authorized
                .load(std::sync::atomic::Ordering::Acquire)
            {
                if let Err(error) =
                    crate::managed_agents::persona_events::flush_pending_events(&db_path, &state)
                        .await
                {
                    eprintln!("buzz-desktop: managed event-flush: {error}");
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
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
async fn current_credentials(
    state: &EvaosTeamsState,
) -> Result<(Zeroizing<String>, Keys, bool), String> {
    initialize_runtime(state)?;
    let runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    let session = runtime
        .session
        .clone()
        .ok_or_else(|| "Sign in to Hive first".to_string())?;
    let keys = runtime
        .keys
        .clone()
        .ok_or_else(|| "Managed identity is unavailable".to_string())?;
    Ok((session, keys, runtime.logout_pending))
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
fn persist_signed_out_identities() -> Result<(), String> {
    let mut replacement = managed_store().load_all_readonly()?.unwrap_or_default();
    replacement.remove(SESSION_KEY);
    replacement.remove(LOGOUT_PENDING_KEY);
    replacement.remove(ACTIVE_MEMBERSHIP_KEY);
    managed_store().replace_all(&replacement)?;
    if managed_store().load_all_readonly()? != Some(replacement) {
        return Err("managed identity read-back verification failed".to_string());
    }
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
            return EvaosTeamsAuthStatus::logout_pending(format!(
                "Remote logout is still pending: {error}"
            ));
        }
    }
    match persist_signed_out_identities() {
        Ok(()) => {
            if let Ok(mut runtime) = state.runtime.lock() {
                *runtime = ManagedRuntime {
                    initialized: true,
                    ..ManagedRuntime::default()
                };
            }
            EvaosTeamsAuthStatus::signed_out()
        }
        Err(error) => EvaosTeamsAuthStatus::locked(error),
    }
}

#[cfg(feature = "evaos-teams-managed")]
async fn begin_managed_logout(
    state: &EvaosTeamsState,
    app_state: &AppState,
) -> Result<EvaosTeamsAuthStatus, String> {
    let (session, keys, _) = current_credentials(state).await?;
    disable_managed_access(app_state);
    let mut pending = managed_store().load_all_readonly()?.unwrap_or_default();
    let membership_id = state
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.membership_id.clone());
    if let Some(membership_id) = membership_id {
        pending.insert(
            membership_identity_key(&membership_id)?,
            encode_managed_identity(&keys)?,
        );
        pending.insert(ACTIVE_MEMBERSHIP_KEY.to_string(), membership_id);
    } else {
        pending.insert(IDENTITY_KEY.to_string(), encode_managed_identity(&keys)?);
    }
    pending.insert(SESSION_KEY.to_string(), session.to_string());
    pending.insert(LOGOUT_PENDING_KEY.to_string(), "1".to_string());
    managed_store()
        .replace_all(&pending)
        .map_err(|_| "Could not record durable managed logout".to_string())?;
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
fn persist_managed_credentials(
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: String,
    keys: Keys,
    membership_id: String,
    entitlement: EvaosTeamsEntitlement,
) -> Result<EvaosTeamsAuthStatus, String> {
    let replacement = managed_credential_entries(
        managed_store().load_all_readonly()?.unwrap_or_default(),
        &membership_id,
        &keys,
        &session,
    )?;
    managed_store()
        .replace_all(&replacement)
        .map_err(|_| "Could not save managed access in macOS Keychain".to_string())?;
    if managed_store().load_all_readonly()? != Some(replacement) {
        return Err("Managed Keychain read-back verification failed".to_string());
    }
    install_entitlement(app_state, &keys, &entitlement)?;
    {
        let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
        *runtime = ManagedRuntime {
            initialized: true,
            session: Some(Zeroizing::new(session)),
            keys: Some(keys),
            membership_id: Some(membership_id),
            logout_pending: false,
        };
    }
    Ok(EvaosTeamsAuthStatus::active(entitlement))
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
        let (session, keys, logout_pending) = match credentials {
            Ok(value) => value,
            Err(_) => {
                disable_managed_access(&app_state);
                return Ok(EvaosTeamsAuthStatus::signed_out());
            }
        };
        if logout_pending {
            return Ok(
                retry_pending_logout(&app_state.http_client, &state, &app_state, &session).await,
            );
        }
        let response = get_remote_entitlement(&app_state.http_client, &session).await;
        match response {
            Ok(entitlement) => {
                if let Err(error) = install_entitlement(&app_state, &keys, &entitlement) {
                    disable_managed_access(&app_state);
                    return Ok(EvaosTeamsAuthStatus::reauth(error));
                }
                Ok(EvaosTeamsAuthStatus::active(entitlement))
            }
            Err(error) => {
                disable_managed_access(&app_state);
                Ok(EvaosTeamsAuthStatus::reauth(format!(
                    "Managed access could not be refreshed: {error}"
                )))
            }
        }
    }
}

/// Return only the company-scoped public agent catalog. Room, member, seat,
/// and control-plane data from the collaboration projection never crosses the
/// Tauri boundary. Relay profiles remain the source of live status/channels.
#[tauri::command]
pub(crate) async fn list_hive_company_agents(
    state: State<'_, EvaosTeamsState>,
    app_state: State<'_, AppState>,
) -> Result<Vec<HiveCompanyAgent>, String> {
    #[cfg(not(feature = "evaos-teams-managed"))]
    {
        let _ = (&state, &app_state);
        Ok(Vec::new())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        if !app_state
            .evaos_teams_authorized
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("Hive access is not active".to_string());
        }
        let (session, _, logout_pending) = current_credentials(&state).await?;
        if logout_pending {
            return Err("Hive sign-out is still pending".to_string());
        }
        let response: CollaborationResponse = post_json(
            &app_state.http_client,
            "evaos-teams-access",
            Some(&session),
            serde_json::json!({ "action": "get_collaboration_state" }),
        )
        .await
        .map_err(|error| format!("Company agent catalog is unavailable: {error}"))?;
        if response.status != "active" {
            return Err("Company agent catalog is inactive".to_string());
        }
        Ok(sanitize_company_agents(response.collaboration.agents))
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
    let received_state = query.get("desktop_auth_state").map(String::as_str);
    let received_code = query.get("device_code").map(String::as_str);
    if received_state != Some(expected_state) {
        return Err("Authentication callback did not match this login attempt".to_string());
    }
    if let Some(received_code) = received_code {
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
    let result = callback_device_code(&query, &state.expected_state);
    match result {
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
    if challenge.challenge.membership_id != expected_membership_id {
        return Err("Managed key challenge changed the selected membership".to_string());
    }
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
    membership_identity_key(&response.binding.membership_id)?;
    Ok(response.binding)
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
        Err("Hive managed login is not enabled in this build".to_string())
    }

    #[cfg(feature = "evaos-teams-managed")]
    {
        let _operation = state.operation.lock().await;
        // Prove Keychain reachability before opening the browser. Managed mode
        // never falls back to a plaintext identity or Desktop token.
        verify_managed_store_writable()?;

        initialize_runtime(&state)?;
        if let Ok((session, keys, logout_pending)) = current_credentials(&state).await {
            if logout_pending {
                let outcome = begin_managed_logout(&state, &app_state).await?;
                if outcome.phase != "signed_out" {
                    return Ok(outcome);
                }
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
                        // A stale token cannot safely authorize work, but it does
                        // not own the durable identity. The server-selected
                        // membership below chooses its own Keychain key.
                        let _ = keys;
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
        let binding = get_identity_binding(&app_state.http_client, &claim.desktop_session).await?;
        let stored = managed_store().load_all_readonly()?.unwrap_or_default();
        let candidate_keys = select_login_keys(&stored, &binding)?;
        let entitlement = bind_identity(
            &app_state.http_client,
            &claim.desktop_session,
            &candidate_keys,
            &binding.membership_id,
        )
        .await?;
        let rollback_session = claim.desktop_session.clone();
        let result = persist_managed_credentials(
            &state,
            &app_state,
            claim.desktop_session,
            candidate_keys,
            binding.membership_id,
            entitlement,
        );
        if result.is_err() {
            let _ = remote_logout(&app_state.http_client, &rollback_session).await;
        }
        result
    }
}

/// Revoke only the current server session grant. The durable Hive identity and
/// its private key remain in Keychain so offline messages keep their recipient
/// and the next login can prove possession without rotating identity.
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
