use std::sync::atomic::Ordering;

use nostr::Keys;

use super::AppState;

impl AppState {
    /// Revoke every in-process capability derived from a managed entitlement.
    /// This is also called when `signing_keys` discovers backend expiry, so a
    /// long-lived huddle cannot outlive the authorization that started it.
    #[cfg(feature = "evaos-teams-managed")]
    pub(crate) fn disable_evaos_teams_access(&self) {
        self.disable_evaos_teams_access_if_current(None);
    }

    #[cfg(feature = "evaos-teams-managed")]
    fn disable_evaos_teams_access_if_current(&self, expected: Option<(u64, i64)>) -> bool {
        let app = self
            .app_handle
            .lock()
            .ok()
            .and_then(|handle| handle.clone());
        let old_pipelines = {
            let _transition = self
                .evaos_teams_access_transition
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some((generation, expires_at)) = expected {
                let still_current = self.evaos_teams_access_generation.load(Ordering::Acquire)
                    == generation
                    && self.evaos_teams_expires_at.load(Ordering::Acquire) == expires_at;
                if !still_current {
                    return false;
                }
            }
            self.evaos_teams_access_generation
                .fetch_add(1, Ordering::AcqRel);
            self.evaos_teams_authorized.store(false, Ordering::Release);
            self.evaos_teams_expires_at.store(0, Ordering::Release);
            if let Ok(mut relay) = self.relay_url_override.lock() {
                *relay = None;
            }
            self.huddle_state.lock().ok().map(|mut huddle| {
                huddle.session_generation.fetch_add(1, Ordering::Release);
                if let Some(cancel) = huddle.audio_ws_cancel.take() {
                    cancel.cancel();
                }
                huddle.audio_relay_pcm_tx.take();
                let stt = huddle.stt_pipeline.take();
                let tts = huddle.tts_pipeline.take();
                huddle.reset_preserving_generation();
                (stt, tts)
            })
        };
        drop(old_pipelines);
        let _runtime_transition = self
            .managed_agent_runtime_transition
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let mut runtimes = self
            .managed_agent_processes
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let runtime_keys = runtimes.keys().cloned().collect::<Vec<_>>();
        for key in runtime_keys {
            let Some(mut runtime) = runtimes.remove(&key) else {
                continue;
            };
            let stop_result = if crate::managed_agents::process_is_running(runtime.child.id()) {
                crate::managed_agents::terminate_process(runtime.child.id())
            } else {
                Ok(())
            }
            .and_then(|()| runtime.child.wait().map_err(|error| error.to_string()));
            match stop_result {
                Ok(_) => {
                    self.clear_agent_session_cache(&key);
                    if let Some(app) = app.as_ref() {
                        crate::managed_agents::remove_agent_runtime_receipt(app, &key);
                    }
                }
                Err(error) => {
                    eprintln!(
                        "evaos-teams: failed to stop revoked native agent runtime {}: {error}",
                        key.pubkey
                    );
                    runtimes.insert(key, runtime);
                }
            }
        }
        drop(runtimes);
        self.emit_huddle_state_changed();
        true
    }

    /// Install a validated managed capability and arm an entitlement-owned
    /// expiry task. The transition lock and generation token make refresh and
    /// expiry mutually exclusive; the timer is independent of huddle sockets.
    #[cfg(feature = "evaos-teams-managed")]
    fn install_evaos_teams_access_state(
        &self,
        keys: Keys,
        relay: String,
        expires_at: i64,
        now: i64,
    ) -> Result<u64, String> {
        let _transition = self
            .evaos_teams_access_transition
            .lock()
            .map_err(|error| error.to_string())?;
        let mut current_keys = self.keys.lock().map_err(|error| error.to_string())?;
        let mut current_relay = self
            .relay_url_override
            .lock()
            .map_err(|error| error.to_string())?;
        let same_authority = self.evaos_teams_authorized.load(Ordering::Acquire)
            && self.evaos_teams_expires_at.load(Ordering::Acquire) > now
            && current_keys.public_key() == keys.public_key()
            && current_relay.as_deref() == Some(relay.as_str());

        let generation = if same_authority {
            self.evaos_teams_access_generation.load(Ordering::Acquire)
        } else {
            // Invalidate existing sockets before changing any authority state.
            // Their inbound loop observes this generation with Acquire ordering
            // and emits a content-free terminal callback before renderer delivery.
            let generation = self
                .evaos_teams_access_generation
                .fetch_add(1, Ordering::AcqRel)
                .wrapping_add(1);
            self.evaos_teams_authorized.store(false, Ordering::Release);
            *current_keys = keys;
            *current_relay = Some(relay);
            generation
        };
        self.evaos_teams_expires_at
            .store(expires_at, Ordering::Release);
        self.evaos_teams_authorized.store(true, Ordering::Release);
        Ok(generation)
    }

    #[cfg(feature = "evaos-teams-managed")]
    pub(crate) fn install_evaos_teams_access(
        &self,
        keys: Keys,
        relay: String,
        expires_at: i64,
    ) -> Result<(), String> {
        use tauri::Manager;

        let app = self
            .app_handle
            .lock()
            .map_err(|error| error.to_string())?
            .clone()
            .ok_or_else(|| "managed entitlement expiry task is unavailable".to_string())?;
        let generation = self.install_evaos_teams_access_state(
            keys,
            relay,
            expires_at,
            chrono::Utc::now().timestamp(),
        )?;

        tauri::async_runtime::spawn(async move {
            loop {
                let wait_seconds =
                    u64::try_from(expires_at.saturating_sub(chrono::Utc::now().timestamp()))
                        .unwrap_or(0);
                if wait_seconds > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(wait_seconds)).await;
                    continue;
                }
                let state = app.state::<AppState>();
                state.disable_evaos_teams_access_if_current(Some((generation, expires_at)));
                break;
            }
        });
        Ok(())
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
        loop {
            let now = chrono::Utc::now().timestamp();
            let invalid_snapshot = {
                let _transition = self
                    .evaos_teams_access_transition
                    .lock()
                    .unwrap_or_else(|error| error.into_inner());
                let generation = self.evaos_teams_access_generation.load(Ordering::Acquire);
                let expires_at = self.evaos_teams_expires_at.load(Ordering::Acquire);
                (!self.evaos_teams_authorized.load(Ordering::Acquire) || expires_at <= now)
                    .then_some((generation, expires_at))
            };
            let Some(expected) = invalid_snapshot else {
                break;
            };
            if self.disable_evaos_teams_access_if_current(Some(expected)) {
                return Err(
                    "Hive access is not currently authorized; sign in or refresh access"
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

#[cfg(all(test, feature = "evaos-teams-managed"))]
mod tests {
    use super::*;
    use crate::app_state::build_app_state;

    #[test]
    fn identical_refresh_preserves_generation_but_authority_change_invalidates_first() {
        let state = build_app_state();
        let keys = Keys::generate();
        let now = 1_000;
        let first_generation = state
            .install_evaos_teams_access_state(
                keys.clone(),
                "wss://relay-a.example.com".to_string(),
                now + 60,
                now,
            )
            .unwrap();
        assert_eq!(first_generation, 1);

        let refresh_generation = state
            .install_evaos_teams_access_state(
                keys,
                "wss://relay-a.example.com".to_string(),
                now + 120,
                now + 30,
            )
            .unwrap();
        assert_eq!(refresh_generation, first_generation);
        assert_eq!(
            state.evaos_teams_access_generation.load(Ordering::Acquire),
            first_generation
        );
        assert!(state.evaos_teams_authorized.load(Ordering::Acquire));

        let switched_generation = state
            .install_evaos_teams_access_state(
                Keys::generate(),
                "wss://relay-b.example.com".to_string(),
                now + 180,
                now + 40,
            )
            .unwrap();
        assert_eq!(switched_generation, first_generation + 1);
        assert_eq!(
            state.evaos_teams_access_generation.load(Ordering::Acquire),
            switched_generation
        );
        assert!(state.evaos_teams_authorized.load(Ordering::Acquire));
        assert_eq!(
            state.relay_url_override.lock().unwrap().as_deref(),
            Some("wss://relay-b.example.com")
        );
    }
}
