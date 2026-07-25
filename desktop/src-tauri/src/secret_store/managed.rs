use std::collections::HashMap;

use super::SecretStore;

fn replace_entries(current: &mut HashMap<String, String>, replacement: &HashMap<String, String>) {
    current.clear();
    current.extend(replacement.clone());
}

fn rewrite_current_with<R, W>(read: R, write: W) -> Result<HashMap<String, String>, String>
where
    R: FnOnce() -> Result<Option<Vec<u8>>, String>,
    W: FnOnce(&[u8]) -> Result<(), String>,
{
    let current = match read()? {
        None => HashMap::new(),
        Some(bytes) => {
            let json = String::from_utf8(bytes).map_err(|error| format!("blob utf8: {error}"))?;
            serde_json::from_str::<HashMap<String, String>>(&json)
                .map_err(|error| format!("blob json: {error}"))?
        }
    };
    let json =
        serde_json::to_string(&current).map_err(|error| format!("blob serialize: {error}"))?;
    write(json.as_bytes())?;
    Ok(current)
}

impl SecretStore {
    /// Atomically replace the managed keychain blob with exactly `entries`.
    pub fn replace_all(&self, entries: &HashMap<String, String>) -> Result<(), String> {
        self.mutate_blob(|map| replace_entries(map, entries))
    }

    /// Prove that the managed keychain entry is writable without changing its
    /// semantic contents. Unlike `mutate_blob`, this deliberately performs the
    /// write even when the freshly read map is unchanged so macOS validates the
    /// caller's write ACL before browser authentication creates a remote
    /// session.
    pub fn force_rewrite_current(&self) -> Result<HashMap<String, String>, String> {
        let _lock = super::acquire_blob_lock(&self.service)?;
        match rewrite_current_with(|| self.read_blob_raw(), |bytes| self.write_blob_raw(bytes)) {
            Ok(current) => {
                let mut cache = self.cache.lock().unwrap_or_else(|error| error.into_inner());
                *cache = Some(current.clone());
                Ok(current)
            }
            Err(error) => {
                let mut cache = self.cache.lock().unwrap_or_else(|error| error.into_inner());
                *cache = None;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_removes_prior_account_secrets() {
        let mut current = HashMap::from([
            ("identity".to_string(), "old-identity".to_string()),
            (
                "electric_desktop_session".to_string(),
                "old-session".to_string(),
            ),
            ("unrelated".to_string(), "must-not-survive".to_string()),
        ]);
        let replacement = HashMap::from([
            ("identity".to_string(), "new-identity".to_string()),
            (
                "electric_desktop_session".to_string(),
                "new-session".to_string(),
            ),
        ]);
        replace_entries(&mut current, &replacement);
        assert_eq!(current, replacement);
    }

    #[test]
    fn writable_probe_forces_an_identical_or_empty_blob_write() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let writes = AtomicUsize::new(0);
        let original = HashMap::from([("identity".to_string(), "nsec-test".to_string())]);
        let encoded = serde_json::to_vec(&original).unwrap();
        let rewritten = rewrite_current_with(
            || Ok(Some(encoded)),
            |bytes| {
                writes.fetch_add(1, Ordering::Relaxed);
                assert_eq!(
                    serde_json::from_slice::<HashMap<String, String>>(bytes).unwrap(),
                    original
                );
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(rewritten, original);
        assert_eq!(writes.load(Ordering::Relaxed), 1);

        let empty_writes = AtomicUsize::new(0);
        let empty = rewrite_current_with(
            || Ok(None),
            |bytes| {
                empty_writes.fetch_add(1, Ordering::Relaxed);
                assert_eq!(bytes, b"{}");
                Ok(())
            },
        )
        .unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty_writes.load(Ordering::Relaxed), 1);
    }
}
