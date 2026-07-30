use super::*;
use crate::app_state::managed_identity::{
    ensure_managed_identity_unchanged, persist_managed_identity_to_keyring,
};

#[test]
fn managed_recovery_requires_keyring_and_never_creates_plaintext_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("identity.key");
    let keys = Keys::generate();
    let store = FakeIdentityStore::store_failing();

    let error =
        persist_managed_identity_to_keyring(&store, &keys, &legacy_path, dir.path()).unwrap_err();

    assert!(error.contains("simulated keyring write failure"));
    assert!(!legacy_path.exists());
    assert!(migration_marker_path(dir.path()).exists());
}

#[test]
fn managed_recovery_verifies_keyring_before_removing_old_identity_file() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("identity.key");
    let old_keys = Keys::generate();
    save_key_file(&legacy_path, &old_keys).unwrap();
    let recovered = Keys::generate();
    let store = FakeIdentityStore::with_verify_failing();

    assert!(
        persist_managed_identity_to_keyring(&store, &recovered, &legacy_path, dir.path()).is_err()
    );
    assert!(legacy_path.exists());

    let store = FakeIdentityStore::reachable_but_empty();
    persist_managed_identity_to_keyring(&store, &recovered, &legacy_path, dir.path()).unwrap();
    assert!(!legacy_path.exists());
    assert_eq!(
        store
            .slot
            .borrow()
            .get(IDENTITY_KEY_NAME)
            .map(String::as_str),
        Some(recovered.secret_key().to_bech32().unwrap().as_str())
    );
}

#[cfg(feature = "evaos-teams-managed")]
#[test]
fn managed_boot_keeps_verified_keyring_identity_over_stale_plaintext_file() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("identity.key");
    let stale = Keys::generate();
    save_key_file(&legacy_path, &stale).unwrap();
    let recovered = Keys::generate();
    let store = FakeIdentityStore::present_with(&recovered.secret_key().to_bech32().unwrap());

    let resolved = resolve_identity_with_store(&store, &legacy_path, dir.path()).unwrap();

    assert_key_eq(&resolved.keys, &recovered);
    assert!(!legacy_path.exists());
}

#[test]
fn managed_recovery_rejects_an_identity_changed_during_network_exchange() {
    let before = Keys::generate();
    let replacement = Keys::generate();
    let expected = before.public_key().to_hex();

    ensure_managed_identity_unchanged(&before, &expected).unwrap();
    assert!(ensure_managed_identity_unchanged(&replacement, &expected).is_err());
}
