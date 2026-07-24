use std::sync::atomic::Ordering;

use nostr::Keys;

use super::AppState;

impl AppState {
    /// Revoke every in-process capability derived from a managed entitlement.
    /// This is also called when `signing_keys` discovers backend expiry, so a
    /// long-lived huddle cannot outlive the authorization that started it.
    #[cfg(feature = "evaos-teams-managed")]
    pub(crate) fn disable_evaos_teams_access(&self) {
        self.evaos_teams_authorized.store(false, Ordering::Release);
        self.evaos_teams_expires_at.store(0, Ordering::Release);
        if let Ok(mut relay) = self.relay_url_override.lock() {
            *relay = None;
        }
        let old_pipelines = self.huddle_state.lock().ok().map(|mut huddle| {
            huddle.session_generation.fetch_add(1, Ordering::Release);
            if let Some(cancel) = huddle.audio_ws_cancel.take() {
                cancel.cancel();
            }
            huddle.audio_relay_pcm_tx.take();
            let stt = huddle.stt_pipeline.take();
            let tts = huddle.tts_pipeline.take();
            huddle.reset_preserving_generation();
            (stt, tts)
        });
        drop(old_pipelines);
        self.emit_huddle_state_changed();
    }

    /// Return the active identity keys if they are in a signable state.
    ///
    /// Managed builds additionally require a current broker entitlement.
    /// Native recovery mode blocks publishing under an invalid or inaccessible
    /// identity until the identity is restored and Buzz is relaunched.
    pub fn signing_keys(&self) -> Result<Keys, String> {
        if self.identity_lost.load(Ordering::Acquire) || self.keyring_locked.load(Ordering::Acquire)
        {
            return Err("identity is in recovery mode; event signing is disabled \
                 until the identity is restored and Buzz is relaunched"
                .to_string());
        }
        #[cfg(feature = "evaos-teams-managed")]
        {
            let now = chrono::Utc::now().timestamp();
            if !self.evaos_teams_authorized.load(Ordering::Acquire)
                || self.evaos_teams_expires_at.load(Ordering::Acquire) <= now
            {
                self.disable_evaos_teams_access();
                return Err(
                    "evaOS Teams access is not currently authorized; sign in or refresh access"
                        .to_string(),
                );
            }
        }
        self.keys
            .lock()
            .map_err(|e| e.to_string())
            .map(|keys| keys.clone())
    }
}
