use super::*;
use tauri::Manager;

fn persist_active_session(
    state: &EvaosTeamsState,
    app_state: &AppState,
    session: String,
    keys: &Keys,
    membership_id: &str,
    entitlement: EvaosTeamsEntitlement,
) -> Result<EvaosTeamsAuthStatus, String> {
    let mut runtime = state.runtime.lock().map_err(|error| error.to_string())?;
    install_entitlement(app_state, keys, &entitlement)?;
    if managed_store()
        .replace_all_checked(|fresh| {
            keychain_migration::active_session_entries(
                fresh,
                &session,
                membership_id,
                &keys.public_key().to_hex(),
            )
        })
        .is_err()
    {
        disable_managed_access(app_state);
        return Err("Could not commit managed access in macOS Keychain".to_string());
    }
    *runtime = ManagedRuntime {
        initialized: true,
        session: Some(Zeroizing::new(session)),
        logout_pending: false,
        custody_checked: true,
    };
    Ok(EvaosTeamsAuthStatus::active(entitlement))
}

pub(super) fn custody_checked(state: &EvaosTeamsState) -> Result<bool, String> {
    initialize_runtime(state)?;
    state
        .runtime
        .lock()
        .map_err(|error| error.to_string())
        .map(|runtime| runtime.custody_checked)
}

pub(super) fn mark_custody_checked(state: &EvaosTeamsState) -> Result<(), String> {
    initialize_runtime(state)?;
    state
        .runtime
        .lock()
        .map_err(|error| error.to_string())?
        .custody_checked = true;
    Ok(())
}

pub(super) fn binding_for_entitlement(
    binding: &IdentityBinding,
    entitlement: &EvaosTeamsEntitlement,
) -> Result<IdentityBinding, String> {
    let public_key = entitlement
        .public_key
        .clone()
        .ok_or_else(|| "Managed entitlement did not include the verified identity".to_string())?;
    Ok(IdentityBinding {
        membership_id: binding.membership_id.clone(),
        public_key: Some(public_key),
    })
}

pub(super) async fn complete_login(
    app: &tauri::AppHandle,
    state: &EvaosTeamsState,
    app_state: &AppState,
    desktop_session: String,
) -> Result<EvaosTeamsAuthStatus, String> {
    let binding = get_identity_binding(&app_state.http_client, &desktop_session).await?;
    let local_keys = native_identity_for_managed_verification(app_state);
    let local_identity_matches = local_keys
        .as_ref()
        .ok()
        .and_then(|keys| verify_existing_native_identity(&binding, keys).ok());
    let (keys, entitlement) = if local_identity_matches.is_some() {
        let keys = local_keys
            .map_err(|error| format!("The local Hive identity could not be verified: {error}"))?;
        let entitlement = bind_identity(
            &app_state.http_client,
            &desktop_session,
            &keys,
            &binding.membership_id,
        )
        .await?;
        let verified_binding = binding_for_entitlement(&binding, &entitlement)?;
        identity_custody::ensure_enrollment(
            &app_state.http_client,
            &desktop_session,
            &verified_binding,
            &entitlement,
            &keys,
        )
        .await?;
        (keys, entitlement)
    } else if binding.public_key.is_some() {
        let canonical_public_key = binding
            .public_key
            .as_deref()
            .ok_or_else(|| "Managed identity binding is missing".to_string())?;
        let legacy_candidate = keychain_migration::select_legacy_identity_candidate(
            &managed_store().load_all_readonly()?.unwrap_or_default(),
            &binding.membership_id,
            canonical_public_key,
        )?;
        if let Some(keys) = legacy_candidate {
            let entitlement = bind_identity(
                &app_state.http_client,
                &desktop_session,
                &keys,
                &binding.membership_id,
            )
            .await?;
            let verified_binding = binding_for_entitlement(&binding, &entitlement)?;
            identity_custody::ensure_enrollment(
                &app_state.http_client,
                &desktop_session,
                &verified_binding,
                &entitlement,
                &keys,
            )
            .await?;
            authorization::prepare_managed_identity_recovery(app, app_state)?;
            let expected_local_public_key = app_state
                .keys
                .lock()
                .map_err(|error| error.to_string())?
                .public_key()
                .to_hex();
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| format!("app data dir: {error}"))?;
            std::fs::create_dir_all(&data_dir)
                .map_err(|error| format!("create app data dir: {error}"))?;
            let key_path = data_dir.join("identity.key");
            let store =
                crate::secret_store::SecretStore::shared(crate::app_state::keyring_service());
            crate::app_state::managed_identity::persist_managed_recovered_identity(
                store,
                app_state,
                &keys,
                &expected_local_public_key,
                &key_path,
                &data_dir,
            )?;
            (keys, entitlement)
        } else {
            identity_custody::recover_identity(
                app,
                app_state,
                &app_state.http_client,
                &desktop_session,
                &binding,
            )
            .await?
        }
    } else {
        return Err(
            "Hive could not establish a native identity for this new membership".to_string(),
        );
    };
    persist_active_session(
        state,
        app_state,
        desktop_session,
        &keys,
        &binding.membership_id,
        entitlement,
    )
}
