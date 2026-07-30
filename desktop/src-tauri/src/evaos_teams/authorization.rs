use std::sync::atomic::Ordering;

use nostr::Keys;

use crate::app_state::AppState;

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
