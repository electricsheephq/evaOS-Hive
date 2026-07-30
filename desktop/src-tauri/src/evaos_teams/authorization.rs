use std::sync::atomic::Ordering;

use nostr::Keys;

use crate::app_state::AppState;

/// Require an installed server-selected relay entitlement before managed
/// builds may sign, publish, or read protected collaboration state.
pub(crate) fn require_managed_authorization(app_state: &AppState) -> Result<(), String> {
    if !cfg!(feature = "evaos-teams-managed")
        || app_state
            .relay_url_override
            .lock()
            .map_err(|error| error.to_string())?
            .is_some()
    {
        return Ok(());
    }
    Err("Hive access is not authorized; complete Electric Sheep sign-in first".to_string())
}

/// Reject in-process identity replacement while a managed entitlement is
/// active. Recovery while signed out remains available to the auth gate.
pub(crate) fn require_managed_identity_recovery(app_state: &AppState) -> Result<(), String> {
    if cfg!(feature = "evaos-teams-managed")
        && app_state
            .relay_url_override
            .lock()
            .map_err(|error| error.to_string())?
            .is_some()
    {
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

    #[test]
    fn managed_identity_recovery_is_blocked_while_entitlement_is_active() {
        let state = crate::app_state::build_app_state();
        *state.relay_url_override.lock().unwrap() = Some("wss://relay.example.com".to_string());

        assert!(require_managed_identity_recovery(&state).is_err());
    }

    #[test]
    fn managed_identity_recovery_remains_available_without_entitlement() {
        let state = crate::app_state::build_app_state();

        assert!(require_managed_identity_recovery(&state).is_ok());
    }
}
