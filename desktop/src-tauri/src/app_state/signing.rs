use std::sync::atomic::Ordering;

use nostr::Keys;

use super::AppState;

impl AppState {
    /// Return the active identity keys if they are in a signable state.
    ///
    /// Managed builds additionally require a current broker entitlement.
    /// Native recovery mode blocks publishing under an invalid or inaccessible
    /// identity until the identity is restored and Buzz is relaunched.
    pub fn signing_keys(&self) -> Result<Keys, String> {
        #[cfg(feature = "evaos-teams-managed")]
        if !self.evaos_teams_authorized.load(Ordering::Acquire) {
            return Err(
                "evaOS Teams access is not currently authorized; sign in or refresh access"
                    .to_string(),
            );
        }
        if self.identity_lost.load(Ordering::Acquire) || self.keyring_locked.load(Ordering::Acquire)
        {
            return Err("identity is in recovery mode; event signing is disabled \
                 until the identity is restored and Buzz is relaunched"
                .to_string());
        }
        self.keys
            .lock()
            .map_err(|e| e.to_string())
            .map(|keys| keys.clone())
    }
}
