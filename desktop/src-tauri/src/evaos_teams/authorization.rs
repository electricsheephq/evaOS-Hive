use std::sync::atomic::Ordering;

use nostr::Keys;

use crate::app_state::AppState;

pub(super) fn disable_managed_access(app_state: &AppState) {
    if let Ok(mut relay) = app_state.relay_url_override.lock() {
        app_state
            .managed_entitlement_expires_at_unix
            .store(0, Ordering::Release);
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
    app_state
        .managed_entitlement_expires_at_unix
        .store(0, Ordering::Release);
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
        assert_eq!(
            state
                .managed_entitlement_expires_at_unix
                .load(Ordering::Acquire),
            0
        );
    }

    #[test]
    fn expired_entitlement_allows_identity_recovery() {
        let state = crate::app_state::build_app_state();
        install_test_entitlement(&state, chrono::Utc::now().timestamp() - 1);

        assert!(require_managed_identity_recovery(&state).is_ok());
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
