use nostr::Keys;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
#[cfg(not(feature = "evaos-teams-managed"))]
use tauri::Emitter;
use tauri::{AppHandle, Manager, State};

use crate::app_state::AppState;
use crate::managed_agents::restore_managed_agents_on_launch;
#[cfg(not(feature = "evaos-teams-managed"))]
use crate::managed_agents::{
    effective_repos_dir, ensure_repos_symlink, nest_dir, try_regenerate_nest,
    write_persisted_repos_dir,
};
use crate::relay;

#[cfg(any(test, feature = "evaos-teams-managed"))]
fn validate_managed_workspace_request(
    authorized: bool,
    allowed_relay: Option<&str>,
    requested_relay: &str,
    nsec: Option<&str>,
    repos_dir: Option<&str>,
    agent_managed_profiles: Option<bool>,
) -> Result<(), String> {
    if nsec.is_some_and(|value| !value.trim().is_empty()) {
        return Err("Managed workspaces cannot import a private key".to_string());
    }
    if repos_dir.is_some_and(|value| !value.trim().is_empty()) {
        return Err("Managed workspaces cannot override the repositories directory".to_string());
    }
    if agent_managed_profiles.unwrap_or(false) {
        return Err("Managed workspaces cannot enable native agent profile management".to_string());
    }
    if !authorized {
        return Err("Managed workspace access is not authorized".to_string());
    }
    let allowed_relay =
        allowed_relay.ok_or_else(|| "Managed workspace relay is unavailable".to_string())?;
    if requested_relay.trim_end_matches('/') != allowed_relay.trim_end_matches('/') {
        return Err("Managed workspace relay must come from the active entitlement".to_string());
    }
    Ok(())
}

#[cfg(any(test, feature = "evaos-teams-managed"))]
fn validate_managed_workspace_icon_request(
    authorized: bool,
    allowed_relay: Option<&str>,
    requested_relay: &str,
) -> Result<(), String> {
    if !authorized {
        return Err("Managed workspace access is not authorized".to_string());
    }
    let allowed_relay =
        allowed_relay.ok_or_else(|| "Managed workspace relay is unavailable".to_string())?;
    if requested_relay.trim_end_matches('/') != allowed_relay.trim_end_matches('/') {
        return Err(
            "Managed workspace icon relay must come from the active entitlement".to_string(),
        );
    }

    let http_url = relay::relay_http_base_url(requested_relay);
    let parsed = reqwest::Url::parse(&http_url)
        .map_err(|_| "Managed workspace icon relay is invalid".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(
            "Managed workspace icon relay must be a credential-free HTTP origin".to_string(),
        );
    }
    Ok(())
}

#[derive(Deserialize)]
struct RelayInfoIcon {
    #[serde(default)]
    icon: Option<String>,
}

/// Fetch a relay's workspace icon from its NIP-11 relay information document.
///
/// Works for any workspace (active or not) with a plain unauthenticated HTTP
/// GET — no WebSocket session needed. Returns `None` when the relay has no
/// icon set, is unreachable, or serves a malformed document: the rail falls
/// back to initials in all three cases.
#[tauri::command]
pub async fn fetch_workspace_icon(
    relay_url: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    #[cfg(feature = "evaos-teams-managed")]
    {
        // Recheck expiry before allowing any native fetch on behalf of the
        // managed renderer.
        state.signing_keys()?;
        let allowed_relay = state
            .relay_url_override
            .lock()
            .map_err(|error| error.to_string())?
            .clone();
        validate_managed_workspace_icon_request(true, allowed_relay.as_deref(), &relay_url)?;
    }

    let http_url = relay::relay_http_base_url(&relay_url);
    let client = if cfg!(feature = "evaos-teams-managed") {
        // The managed command accepts only the active entitlement relay and
        // must not let that origin redirect the native client to another host.
        &state.media_fetch_client
    } else {
        &state.http_client
    };
    let Ok(response) = client
        .get(&http_url)
        .header("Accept", "application/nostr+json")
        .send()
        .await
    else {
        return Ok(None);
    };
    if !response.status().is_success() {
        return Ok(None);
    }
    let doc = response
        .json::<RelayInfoIcon>()
        .await
        .unwrap_or(RelayInfoIcon { icon: None });
    Ok(doc.icon.filter(|icon| !icon.is_empty()))
}

#[derive(Serialize)]
pub struct ActiveWorkspaceInfo {
    relay_url: String,
    pubkey: String,
}

/// Returns the current active workspace info (relay URL + pubkey).
#[tauri::command]
pub fn get_active_workspace(state: State<'_, AppState>) -> Result<ActiveWorkspaceInfo, String> {
    let keys = state.keys.lock().map_err(|e| e.to_string())?;
    let relay_url = relay::relay_ws_url_with_override(&state);
    Ok(ActiveWorkspaceInfo {
        relay_url,
        pubkey: keys.public_key().to_hex(),
    })
}

/// Validate a candidate `repos_dir` without mutating the filesystem.
///
/// The Add/Edit workspace dialogs call this on submit to block Save on a bad
/// path, so a typo never reaches `apply_workspace`. Reuses the same
/// `validate_repos_dir` the boot/apply path uses — one source of truth for
/// "what's a valid repos dir". An empty/whitespace value clears the override
/// and is valid. `Err` carries the human-readable reason for inline display.
#[tauri::command]
pub async fn validate_repos_dir(dir: String) -> Result<(), String> {
    #[cfg(feature = "evaos-teams-managed")]
    {
        let _ = dir;
        return Err("Managed workspaces cannot override the repositories directory".to_string());
    }
    #[cfg(not(feature = "evaos-teams-managed"))]
    tokio::task::spawn_blocking(move || {
        let trimmed = dir.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let nest = nest_dir().ok_or("cannot resolve home directory for nest")?;
        crate::managed_agents::validate_repos_dir(&nest, trimmed).map(|_| ())
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))?
}

/// Apply a workspace's configuration to the backend session.
///
/// Called by the frontend on app init (after reload) to configure the
/// Tauri backend with the selected workspace's relay URL, keys, and repos
/// directory.
///
/// A bad `repos_dir` is non-fatal: relay/keys always apply (the relay is the
/// active workspace's own choice — orthogonal to the filesystem repos dir),
/// the bad value is NOT persisted (so the next boot starts clean), the
/// `REPOS` symlink is skipped (REPOS stays a real dir), a `repos-dir-error`
/// event surfaces the reason, and the command returns `Ok`. The dialogs
/// already block a bad path at Save (`validate_repos_dir`); this fallback only
/// catches a value that went bad after save (deleted dir, unmounted volume).
#[tauri::command]
pub async fn apply_workspace(
    relay_url: String,
    nsec: Option<String>,
    repos_dir: Option<String>,
    agent_managed_profiles: Option<bool>,
    app: AppHandle,
) -> Result<(), String> {
    let restore_app = app.clone();
    tokio::task::spawn_blocking(move || {
        let state = app.state::<AppState>();

        // ── Validate before mutating ──────────────────────────────────────────
        #[cfg(feature = "evaos-teams-managed")]
        {
            let authorized = state
                .evaos_teams_authorized
                .load(std::sync::atomic::Ordering::Acquire);
            let allowed_relay = state
                .relay_url_override
                .lock()
                .map_err(|error| error.to_string())?
                .clone();
            validate_managed_workspace_request(
                authorized,
                allowed_relay.as_deref(),
                &relay_url,
                nsec.as_deref(),
                repos_dir.as_deref(),
                agent_managed_profiles,
            )?;
        }
        let parsed_keys = match nsec.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(nsec_trimmed) => {
                Some(Keys::parse(nsec_trimmed).map_err(|e| format!("invalid nsec: {e}"))?)
            }
            None => None,
        };

        // Decide the effective repos_dir from the candidate. A bad path does NOT
        // reject — it is treated as if no override were set: relay/keys still
        // apply, the bad value is not persisted, and a `repos-dir-error` surfaces
        // the reason. Persisting a bad path would make every later boot read it,
        // fail to resolve the symlink, and silently skip agent restore. One
        // validate (inside `effective_repos_dir`) drives both the emit and the
        // persisted value. `nest` is resolved softly: when absent there is nothing
        // to persist or symlink, and relay/keys must still apply unconditionally.
        #[cfg(not(feature = "evaos-teams-managed"))]
        let nest = nest_dir();
        #[cfg(not(feature = "evaos-teams-managed"))]
        let effective_repos_dir = match nest.as_deref() {
            Some(nest) => match effective_repos_dir(nest, repos_dir.as_deref()) {
                Ok(value) => value,
                Err(error) => {
                    let _ = app.emit("repos-dir-error", error);
                    None
                }
            },
            None => None,
        };

        // ── Apply all state changes (nothing below can fail) ──────────────────
        #[cfg(not(feature = "evaos-teams-managed"))]
        {
            let mut override_guard = state.relay_url_override.lock().map_err(|e| e.to_string())?;
            *override_guard = Some(relay_url);
        }
        // Reset the Rust-side admission gate when switching workspace/community,
        // matching `resetRateLimitGate()` on the TS side (useCommunityInit.ts:38).
        #[cfg(not(feature = "evaos-teams-managed"))]
        crate::relay_admission::reset_gate_for_workspace_change();

        if let Some(keys) = parsed_keys {
            let mut keys_guard = state.keys.lock().map_err(|e| e.to_string())?;
            *keys_guard = keys;
        }

        // Keep the backend-side reconcile guard aligned with the frontend
        // experiment before launch-time restore can spawn any agents. Missing
        // means the stable behavior: desktop remains authoritative.
        #[cfg(not(feature = "evaos-teams-managed"))]
        state
            .managed_agent_profile_reconcile_enabled
            .store(!agent_managed_profiles.unwrap_or(false), Ordering::Release);

        // ── Filesystem side-effect (non-fatal) ────────────────────────────────
        // Persist the *effective* repos_dir (None when the candidate failed
        // validation) for the backend to read at boot, then re-point REPOS to
        // match. Persisting first makes the dotfile authoritative even if the
        // symlink apply fails here (e.g. a non-empty real REPOS): the next boot
        // reads the persisted value and resolves the symlink before any agent can
        // clone into REPOS. A bad candidate persists `None`, so the next boot is
        // clean and agent restore proceeds. Failure of either must NOT fail the
        // command — relay/keys are already applied. Surface symlink errors via
        // `repos-dir-error`.
        #[cfg(not(feature = "evaos-teams-managed"))]
        {
            if let Some(nest) = nest.as_deref() {
                if let Err(error) = write_persisted_repos_dir(nest, effective_repos_dir.as_deref())
                {
                    eprintln!("buzz-desktop: persist repos dir failed: {error}");
                }
                if let Err(error) = ensure_repos_symlink(nest, effective_repos_dir.as_deref()) {
                    eprintln!("buzz-desktop: repos dir setup failed: {error}");
                    let _ = app.emit("repos-dir-error", error);
                }
            }

            try_regenerate_nest(&app);
        }

        Ok::<(), String>(())
    })
    .await
    .map_err(|e| format!("spawn_blocking failed: {e}"))??;

    let state = restore_app.state::<AppState>();
    let restore_pending = !cfg!(feature = "evaos-teams-managed")
        && state
            .managed_agent_restore_pending
            .swap(false, Ordering::AcqRel);

    // The coordinator starts before React applies the selected workspace, so
    // its startup publication may have used the fallback relay and placeholder
    // identity. Correct it off the command path so an unavailable relay cannot
    // hold the frontend on its loading gate. On initial launch, restore MeshLLM
    // first so a slow stopped-status request cannot overwrite a newly restored
    // serving status, then restore managed agents after the admission identity
    // has been published (or the bounded publication attempt has timed out).
    #[cfg(feature = "mesh-llm")]
    {
        let app = restore_app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            if restore_pending {
                if let Err(error) =
                    crate::commands::mesh_llm::restore_mesh_sharing(&app, &state).await
                {
                    eprintln!("buzz-desktop: failed to restore Share Compute: {error}");
                }
            }
            crate::mesh_llm::publish_current_status_once(&app, "workspace apply").await;
            if restore_pending {
                if let Err(error) =
                    restore_managed_agents_on_launch(&app, &state.shutdown_started).await
                {
                    eprintln!("buzz-desktop: failed to restore managed agents: {error}");
                }
            }
        });
    }

    #[cfg(not(feature = "mesh-llm"))]
    if restore_pending {
        let app = restore_app.clone();
        tauri::async_runtime::spawn(async move {
            let state = app.state::<AppState>();
            if let Err(error) =
                restore_managed_agents_on_launch(&app, &state.shutdown_started).await
            {
                eprintln!("buzz-desktop: failed to restore managed agents: {error}");
            }
        });
    }

    Ok(())
}

#[cfg(test)]
mod managed_tests {
    use super::{validate_managed_workspace_icon_request, validate_managed_workspace_request};

    #[test]
    fn managed_workspace_rejects_private_key_and_relay_injection() {
        assert!(validate_managed_workspace_request(
            true,
            Some("wss://relay.example.com"),
            "wss://relay.example.com",
            Some("nsec1secret"),
            None,
            None,
        )
        .is_err());
        assert!(validate_managed_workspace_request(
            true,
            Some("wss://relay.example.com"),
            "wss://attacker.example.com",
            None,
            None,
            None,
        )
        .is_err());
    }

    #[test]
    fn managed_workspace_accepts_only_the_authorized_relay_without_native_overrides() {
        assert!(validate_managed_workspace_request(
            true,
            Some("wss://relay.example.com"),
            "wss://relay.example.com/",
            None,
            None,
            Some(false),
        )
        .is_ok());
        assert!(validate_managed_workspace_request(
            false,
            Some("wss://relay.example.com"),
            "wss://relay.example.com",
            None,
            None,
            None,
        )
        .is_err());
        assert!(validate_managed_workspace_request(
            true,
            Some("wss://relay.example.com"),
            "wss://relay.example.com",
            None,
            Some("/tmp/attacker-repos"),
            None,
        )
        .is_err());
        assert!(validate_managed_workspace_request(
            true,
            Some("wss://relay.example.com"),
            "wss://relay.example.com",
            None,
            None,
            Some(true),
        )
        .is_err());
    }

    #[test]
    fn managed_workspace_icon_requires_current_authorized_credential_free_relay() {
        assert!(validate_managed_workspace_icon_request(
            true,
            Some("wss://relay.example.com"),
            "wss://relay.example.com/",
        )
        .is_ok());
        assert!(validate_managed_workspace_icon_request(
            false,
            Some("wss://relay.example.com"),
            "wss://relay.example.com",
        )
        .is_err());
        assert!(validate_managed_workspace_icon_request(
            true,
            Some("wss://relay.example.com"),
            "wss://attacker.example.com",
        )
        .is_err());
        assert!(validate_managed_workspace_icon_request(
            true,
            Some("wss://user:password@relay.example.com"),
            "wss://user:password@relay.example.com",
        )
        .is_err());
    }
}
