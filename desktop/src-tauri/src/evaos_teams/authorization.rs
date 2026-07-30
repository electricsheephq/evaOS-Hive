use std::sync::atomic::Ordering;

use nostr::Keys;
use tauri::Manager;

use crate::app_state::AppState;

pub(super) fn disable_managed_access(app_state: &AppState) {
    app_state
        .managed_agent_restore_pending
        .store(false, Ordering::Release);
    app_state
        .managed_entitlement_expires_at_unix
        .store(0, Ordering::Release);
    if let Ok(mut relay) = app_state.relay_url_override.lock() {
        *relay = None;
    }
}

pub(super) fn revoke_managed_access(
    app: &tauri::AppHandle,
    app_state: &AppState,
) -> Result<(), String> {
    disable_managed_access(app_state);
    crate::shutdown::shutdown_managed_agents(app)
        .map_err(|error| format!("a local agent could not be stopped: {error}"))
}

fn scheduled_entitlement_is_expired(
    app_state: &AppState,
    scheduled_expires_at: i64,
    now: i64,
) -> bool {
    scheduled_expires_at <= now
        && app_state
            .managed_entitlement_expires_at_unix
            .load(Ordering::Acquire)
            == scheduled_expires_at
}

pub(super) fn schedule_managed_access_expiry(app_state: &AppState, expires_at: i64) {
    if !cfg!(feature = "evaos-teams-managed") {
        return;
    }
    let app = app_state
        .app_handle
        .lock()
        .ok()
        .and_then(|guard| guard.clone());
    let Some(app) = app else {
        return;
    };
    let delay_seconds = expires_at.saturating_sub(chrono::Utc::now().timestamp());
    let delay = std::time::Duration::from_secs(delay_seconds.max(0) as u64);
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(delay).await;
        let revoke_app = app.clone();
        if let Err(error) = tauri::async_runtime::spawn_blocking(move || {
            let state = revoke_app.state::<AppState>();
            let _identity_guard = state
                .identity_mutation
                .lock()
                .map_err(|error| error.to_string())?;
            if !scheduled_entitlement_is_expired(&state, expires_at, chrono::Utc::now().timestamp())
            {
                return Ok(());
            }
            revoke_managed_access(&revoke_app, &state)
        })
        .await
        .map_err(|error| format!("managed expiry shutdown task failed: {error}"))
        .and_then(|result| result)
        {
            eprintln!("buzz-desktop: managed access expiry cleanup failed: {error}");
        }
    });
}

fn managed_authorization_active(app_state: &AppState) -> Result<bool, String> {
    let mut relay = app_state
        .relay_url_override
        .lock()
        .map_err(|error| error.to_string())?;
    if relay.is_none() {
        return Ok(false);
    }
    let expires_at = app_state
        .managed_entitlement_expires_at_unix
        .load(Ordering::Acquire);
    if expires_at > chrono::Utc::now().timestamp() {
        return Ok(true);
    }
    // Preserve the expired value until the scheduled cleanup consumes it.
    // A renewed entitlement replaces this value, which invalidates that task.
    app_state
        .managed_agent_restore_pending
        .store(false, Ordering::Release);
    *relay = None;
    Ok(false)
}

/// Require an installed server-selected relay entitlement before managed
/// builds may sign, publish, or read protected collaboration state.
pub(crate) fn require_managed_authorization(app_state: &AppState) -> Result<(), String> {
    if !cfg!(feature = "evaos-teams-managed") || managed_authorization_active(app_state)? {
        return Ok(());
    }
    Err("Hive access is not authorized; complete Electric Sheep sign-in first".to_string())
}

/// Reject in-process identity replacement while a managed entitlement is
/// active. Recovery while signed out remains available to the auth gate.
pub(crate) fn require_managed_identity_recovery(app_state: &AppState) -> Result<(), String> {
    if cfg!(feature = "evaos-teams-managed") && managed_authorization_active(app_state)? {
        return Err("Sign out of Hive before restoring a different native identity".to_string());
    }
    Ok(())
}

/// Prepare a managed build for native identity recovery while signed out.
///
/// Recovery may replace the local signing identity, so every local agent must
/// be stopped after authorization is revoked and before the replacement.
pub(crate) fn prepare_managed_identity_recovery(
    app: &tauri::AppHandle,
    app_state: &AppState,
) -> Result<(), String> {
    require_managed_identity_recovery(app_state)?;
    if cfg!(feature = "evaos-teams-managed") {
        revoke_managed_access(app, app_state)?;
    }
    Ok(())
}

/// Return the already-resolved native Buzz identity for managed admission.
///
/// This bypasses only the managed authorization flag so Hive can prove
/// possession during OAuth. Native recovery states still fail closed, and
/// this method never creates, imports, replaces, or persists a key.
pub(super) fn native_identity_for_managed_verification(
    app_state: &AppState,
) -> Result<Keys, String> {
    if app_state.identity_lost.load(Ordering::Acquire)
        || app_state.keyring_locked.load(Ordering::Acquire)
    {
        return Err(
            "the native Buzz identity must be restored before Hive can verify it".to_string(),
        );
    }
    app_state
        .keys
        .lock()
        .map_err(|error| error.to_string())
        .map(|keys| keys.clone())
}

#[cfg(all(test, feature = "evaos-teams-managed"))]
mod tests {
    use super::*;

    fn install_test_entitlement(state: &AppState, expires_at: i64) {
        state
            .managed_entitlement_expires_at_unix
            .store(expires_at, Ordering::Release);
        *state.relay_url_override.lock().unwrap() = Some("wss://relay.example.com".to_string());
    }

    #[test]
    fn managed_identity_recovery_is_blocked_while_entitlement_is_active() {
        let state = crate::app_state::build_app_state();
        install_test_entitlement(&state, chrono::Utc::now().timestamp() + 60);

        assert!(require_managed_identity_recovery(&state).is_err());
    }

    #[test]
    fn managed_identity_recovery_remains_available_without_entitlement() {
        let state = crate::app_state::build_app_state();

        assert!(require_managed_identity_recovery(&state).is_ok());
    }

    #[test]
    fn expired_entitlement_blocks_signing_and_clears_managed_access() {
        let state = crate::app_state::build_app_state();
        install_test_entitlement(&state, chrono::Utc::now().timestamp() - 1);

        assert!(require_managed_authorization(&state).is_err());
        assert!(state.relay_url_override.lock().unwrap().is_none());
        assert!(
            state
                .managed_entitlement_expires_at_unix
                .load(Ordering::Acquire)
                < chrono::Utc::now().timestamp()
        );
    }

    #[test]
    fn expired_entitlement_allows_identity_recovery() {
        let state = crate::app_state::build_app_state();
        install_test_entitlement(&state, chrono::Utc::now().timestamp() - 1);

        assert!(require_managed_identity_recovery(&state).is_ok());
    }

    #[test]
    fn disabling_access_cancels_pending_agent_restore() {
        let state = crate::app_state::build_app_state();
        install_test_entitlement(&state, chrono::Utc::now().timestamp() + 60);
        state
            .managed_agent_restore_pending
            .store(true, Ordering::Release);

        disable_managed_access(&state);

        assert!(!state.managed_agent_restore_pending.load(Ordering::Acquire));
    }

    #[test]
    fn renewed_entitlement_invalidates_an_older_expiry_task() {
        let state = crate::app_state::build_app_state();
        let old_expiry = chrono::Utc::now().timestamp() - 1;
        install_test_entitlement(&state, old_expiry + 120);

        assert!(!scheduled_entitlement_is_expired(
            &state,
            old_expiry,
            chrono::Utc::now().timestamp()
        ));
    }

    #[test]
    fn expired_entitlement_blocks_nip98_http_signing() {
        let state = crate::app_state::build_app_state();
        install_test_entitlement(&state, chrono::Utc::now().timestamp() - 1);

        assert!(crate::relay::build_nip98_auth_header(
            &reqwest::Method::POST,
            "https://relay.example.com/query",
            b"{}",
            &state,
        )
        .is_err());
    }
}
