use nostr::{Keys, ToBech32};
use zeroize::Zeroizing;

use super::{migration_marker_path, write_migration_marker, IdentityKeyStore, IDENTITY_KEY_NAME};

pub(super) fn ensure_managed_identity_unchanged(
    current: &Keys,
    expected_public_key: &str,
) -> Result<(), String> {
    if current.public_key().to_hex() != expected_public_key {
        return Err("Native identity changed during managed recovery".to_string());
    }
    Ok(())
}

/// Persist a managed OAuth-recovered identity to the OS keyring without ever
/// creating a plaintext file fallback.
///
/// The non-secret migration marker is written first. If Keychain persistence
/// then fails, the next boot remains in a recoverable lost/locked state instead
/// of silently generating a replacement identity.
pub(super) fn persist_managed_identity_to_keyring(
    store: &impl IdentityKeyStore,
    keys: &Keys,
    legacy_path: &std::path::Path,
    data_dir: &std::path::Path,
) -> Result<(), String> {
    write_migration_marker(&migration_marker_path(data_dir))?;
    let nsec = Zeroizing::new(
        keys.secret_key()
            .to_bech32()
            .map_err(|error| format!("encode managed identity: {error}"))?,
    );
    store.store(IDENTITY_KEY_NAME, &nsec)?;
    match store.verify_stored(IDENTITY_KEY_NAME, &nsec) {
        Ok(true) => {}
        Ok(false) => return Err("managed Keychain read-back verification failed".to_string()),
        Err(error) => {
            return Err(format!(
                "managed Keychain read-back verification failed: {error}"
            ));
        }
    }
    if legacy_path.exists() {
        std::fs::remove_file(legacy_path)
            .map_err(|error| format!("remove replaced identity.key: {error}"))?;
    }
    Ok(())
}

/// Install an OAuth-authorized canonical identity into the OS keyring and the
/// in-process signer without exposing private material to the renderer.
///
/// The identity may come from the recovery envelope or from the legacy managed
/// Keychain after its public key matches the server binding. Both paths replace
/// only the exact native identity observed before their asynchronous proof.
#[cfg(feature = "evaos-teams-managed")]
pub(crate) fn persist_managed_recovered_identity(
    store: &crate::secret_store::SecretStore,
    state: &super::AppState,
    keys: &Keys,
    expected_public_key: &str,
    legacy_path: &std::path::Path,
    data_dir: &std::path::Path,
) -> Result<(), String> {
    let _mutation_guard = state
        .identity_mutation
        .lock()
        .map_err(|error| error.to_string())?;
    let current = state.keys.lock().map_err(|error| error.to_string())?;
    ensure_managed_identity_unchanged(&current, expected_public_key)?;
    drop(current);
    persist_managed_identity_to_keyring(store, keys, legacy_path, data_dir)?;
    *state.keys.lock().map_err(|error| error.to_string())? = keys.clone();
    state
        .identity_lost
        .store(false, std::sync::atomic::Ordering::Release);
    state
        .keyring_locked
        .store(false, std::sync::atomic::Ordering::Release);
    Ok(())
}
