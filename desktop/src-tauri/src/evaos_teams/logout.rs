use super::*;

fn finish_confirmed_logout(state: &EvaosTeamsState) -> EvaosTeamsAuthStatus {
    let previous = state
        .runtime
        .lock()
        .ok()
        .and_then(|runtime| runtime.previous_session.clone().zip(runtime.keys.clone()));
    if let Some((previous_session, keys)) = previous {
        let restored = managed_credential_entries(&keys, &previous_session, false, false, None);
        return match restored.and_then(|restored| {
            managed_store().replace_all(&restored)?;
            if managed_store().load_all_readonly()? != Some(restored) {
                return Err("Managed Keychain previous-session verification failed".to_string());
            }
            let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
            *runtime = ManagedRuntime {
                initialized: true,
                session: Some(previous_session),
                previous_session: None,
                keys: Some(keys),
                logout_pending: false,
                logout_confirmed: false,
            };
            Ok(())
        }) {
            Ok(()) => EvaosTeamsAuthStatus::reauth(
                "The prior account was preserved. Sign in to that account again.".to_string(),
            ),
            Err(error) => EvaosTeamsAuthStatus::locked(error),
        };
    }
    match wipe_managed_store() {
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

pub(super) async fn retry_pending_logout(
    client: &reqwest::Client,
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: &str,
) -> EvaosTeamsAuthStatus {
    disable_managed_access(app_state);
    let snapshot = state.runtime.lock().ok().and_then(|runtime| {
        runtime.keys.clone().map(|keys| {
            (
                keys,
                runtime.previous_session.clone(),
                runtime.logout_confirmed,
            )
        })
    });
    let Some((keys, previous_session, already_confirmed)) = snapshot else {
        return EvaosTeamsAuthStatus::locked(
            "Managed logout credentials are unavailable".to_string(),
        );
    };
    if !already_confirmed {
        if let Err(error) = remote_logout(client, session).await {
            return EvaosTeamsAuthStatus::logout_pending(format!(
                "Remote logout is still pending: {error}"
            ));
        }
        let confirmed = match managed_credential_entries(
            &keys,
            session,
            false,
            true,
            previous_session.as_deref().map(String::as_str),
        ) {
            Ok(confirmed) => confirmed,
            Err(error) => return EvaosTeamsAuthStatus::locked(error),
        };
        if let Err(error) = managed_store().replace_all(&confirmed).and_then(|()| {
            if managed_store().load_all_readonly()? == Some(confirmed) {
                Ok(())
            } else {
                Err("Managed Keychain logout-confirmation verification failed".to_string())
            }
        }) {
            return EvaosTeamsAuthStatus::locked(format!(
                "Remote logout succeeded, but its local checkpoint failed: {error}"
            ));
        }
        match state.runtime.lock() {
            Ok(mut runtime) => {
                runtime.logout_pending = false;
                runtime.logout_confirmed = true;
            }
            Err(error) => {
                return EvaosTeamsAuthStatus::locked(format!(
                    "Remote logout was confirmed, but runtime recovery is locked: {error}"
                ));
            }
        }
    }
    finish_confirmed_logout(state)
}

pub(super) async fn begin_managed_logout(
    state: &EvaosTeamsState,
    app_state: &AppState,
) -> Result<EvaosTeamsAuthStatus, String> {
    let (session, keys, _, _, _) = current_credentials(state).await?;
    disable_managed_access(app_state);
    let pending = managed_credential_entries(&keys, &session, true, false, None)?;
    managed_store()
        .replace_all(&pending)
        .map_err(|_| "Could not record durable managed logout".to_string())?;
    if let Ok(mut runtime) = state.runtime.lock() {
        runtime.logout_pending = true;
        runtime.logout_confirmed = false;
        runtime.previous_session = None;
    }
    Ok(retry_pending_logout(&app_state.http_client, state, app_state, &session).await)
}
