use super::*;
use crate::app_state::managed_identity::persist_managed_identity_to_keyring;

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
