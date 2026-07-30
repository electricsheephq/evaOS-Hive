use nostr::Keys;

use super::AppState;

impl AppState {
    /// Return the already-resolved native Buzz identity for managed admission.
    ///
    /// This bypasses only the managed authorization flag so Hive can prove
    /// possession during OAuth. Native recovery states still fail closed, and
    /// this method never creates, imports, replaces, or persists a key.
    #[cfg(feature = "evaos-teams-managed")]
    pub(crate) fn native_identity_for_managed_verification(&self) -> Result<Keys, String> {
        if self
            .identity_lost
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .keyring_locked
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(
                "the native Buzz identity must be restored before Hive can verify it".to_string(),
            );
        }
        self.keys
            .lock()
            .map_err(|error| error.to_string())
            .map(|keys| keys.clone())
    }

    /// Return the active identity keys if they are in a signable state.
    ///
    /// Returns `Err` when the identity is in a lost state (`identity_lost`
    /// — ephemeral key, user must re-import their nsec) or when the keyring
    /// is locked (`keyring_locked` — key is held in a keyring that is
    /// unavailable this boot). All signing and publish commands must call
    /// this instead of locking `state.keys` directly, so that recovery mode
    /// blocks publishing under an invalid or inaccessible identity.
    pub fn signing_keys(&self) -> Result<Keys, String> {
        if !self
            .evaos_teams_authorized
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err(
                "Hive access is not authorized; complete Electric Sheep sign-in first".to_string(),
            );
        }
        if self
            .identity_lost
            .load(std::sync::atomic::Ordering::Acquire)
            || self
                .keyring_locked
                .load(std::sync::atomic::Ordering::Acquire)
        {
            return Err("identity is in recovery mode; event signing is disabled \
                 until the identity is restored and Buzz is relaunched"
                .to_string());
        }
        self.keys
            .lock()
            .map_err(|error| error.to_string())
            .map(|keys| keys.clone())
    }
}
