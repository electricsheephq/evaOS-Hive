use super::*;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::secret_store::KeyringProbe;

struct FailingLoadStore {
    slot: RefCell<HashMap<String, String>>,
    deleted: RefCell<Vec<String>>,
}

impl FailingLoadStore {
    fn new() -> Self {
        Self {
            slot: RefCell::new(HashMap::new()),
            deleted: RefCell::new(Vec::new()),
        }
    }
}

impl IdentityKeyStore for FailingLoadStore {
    fn probe(&self, _name: &str) -> KeyringProbe {
        KeyringProbe::Present
    }

    fn load(&self, _name: &str) -> Result<Option<String>, String> {
        Err("simulated keyring read failure".to_string())
    }

    fn store(&self, name: &str, value: &str) -> Result<(), String> {
        self.slot
            .borrow_mut()
            .insert(name.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, name: &str) -> Result<(), String> {
        self.deleted.borrow_mut().push(name.to_string());
        self.slot.borrow_mut().remove(name);
        Ok(())
    }

    fn verify_stored(&self, name: &str, expected: &str) -> Result<bool, String> {
        Ok(self.slot.borrow().get(name).is_some_and(|v| v == expected))
    }
}

#[test]
fn present_keyring_read_failure_boots_keyring_locked_recovery_without_rotating() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("identity.key");
    write_migration_marker(&migration_marker_path(dir.path())).unwrap();

    let store = FailingLoadStore::new();
    let resolved = resolve_identity_with_store(&store, &legacy_path, dir.path()).unwrap();

    assert_eq!(resolved.recovery, RecoveryState::KeyringLocked);
    assert!(!legacy_path.exists(), "ephemeral key must not be persisted");
    assert!(
        store.slot.borrow().is_empty(),
        "keyring must not be rewritten"
    );
    assert!(
        store.deleted.borrow().is_empty(),
        "keyring entry must remain intact"
    );
}

#[test]
fn present_keyring_read_failure_uses_legacy_file_when_no_marker_exists() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("identity.key");
    let file_keys = Keys::generate();
    save_key_file(&legacy_path, &file_keys).unwrap();
    assert!(!migration_marker_path(dir.path()).exists());

    let store = FailingLoadStore::new();
    let resolved = resolve_identity_with_store(&store, &legacy_path, dir.path()).unwrap();

    assert_eq!(
        file_keys.public_key().to_hex(),
        resolved.keys.public_key().to_hex()
    );
    assert_eq!(resolved.recovery, RecoveryState::None);
    assert!(
        legacy_path.exists(),
        "fallback file must remain authoritative"
    );
    assert!(
        store.slot.borrow().is_empty(),
        "keyring must not be rewritten"
    );
    assert!(
        store.deleted.borrow().is_empty(),
        "keyring entry must remain intact"
    );
}

#[test]
fn present_keyring_read_failure_keeps_corrupt_legacy_file_locked() {
    let dir = tempfile::tempdir().unwrap();
    let legacy_path = dir.path().join("identity.key");
    std::fs::write(&legacy_path, "not-a-valid-nsec").unwrap();

    let store = FailingLoadStore::new();
    let resolved = resolve_identity_with_store(&store, &legacy_path, dir.path()).unwrap();

    assert_eq!(resolved.recovery, RecoveryState::KeyringLocked);
    assert!(
        legacy_path.exists(),
        "corrupt fallback must be left untouched"
    );
    assert!(
        store.slot.borrow().is_empty(),
        "keyring must not be rewritten"
    );
    assert!(
        store.deleted.borrow().is_empty(),
        "keyring entry must remain intact"
    );
}
