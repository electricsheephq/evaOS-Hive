use std::collections::HashMap;

#[cfg(feature = "evaos-teams-managed")]
use std::cell::RefCell;

#[cfg(all(feature = "system-keyring", feature = "evaos-teams-managed"))]
use super::acquire_blob_lock;
use super::SecretStore;

fn replace_entries(current: &mut HashMap<String, String>, replacement: &HashMap<String, String>) {
    current.clear();
    current.extend(replacement.clone());
}

fn replace_entries_checked<F>(
    current: &mut HashMap<String, String>,
    normalize: F,
) -> Result<(), String>
where
    F: FnOnce(&HashMap<String, String>) -> Result<HashMap<String, String>, String>,
{
    let replacement = normalize(current)?;
    replace_entries(current, &replacement);
    Ok(())
}

impl SecretStore {
    /// Atomically replace the managed keychain blob with exactly `entries`.
    pub fn replace_all(&self, entries: &HashMap<String, String>) -> Result<(), String> {
        self.mutate_blob(|map| replace_entries(map, entries))
    }

    /// Atomically validate and replace the freshly read managed keychain blob.
    ///
    /// The normalizer runs inside `mutate_blob`'s cross-process lock, so it
    /// cannot overwrite a session or logout change committed after an earlier
    /// read. Returning an error leaves the durable blob unchanged.
    #[cfg(feature = "evaos-teams-managed")]
    pub fn replace_all_checked<F>(&self, normalize: F) -> Result<(), String>
    where
        F: FnOnce(&HashMap<String, String>) -> Result<HashMap<String, String>, String>,
    {
        let outcome = RefCell::new(None);
        self.mutate_blob_verified(|current| {
            let result = replace_entries_checked(current, normalize);
            *outcome.borrow_mut() = Some(result);
        })?;
        outcome
            .into_inner()
            .unwrap_or_else(|| Err("managed keychain normalization did not run".to_string()))
    }

    #[cfg(all(feature = "system-keyring", feature = "evaos-teams-managed"))]
    fn mutate_blob_verified<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut HashMap<String, String>),
    {
        let _lock = acquire_blob_lock(&self.service)?;
        let raw = self.read_blob_raw()?;
        let current = match raw {
            None => HashMap::new(),
            Some(bytes) => {
                let json = String::from_utf8(bytes).map_err(|e| format!("blob utf8: {e}"))?;
                serde_json::from_str::<HashMap<String, String>>(&json)
                    .map_err(|e| format!("blob json: {e}"))?
            }
        };
        let mut next = current.clone();
        f(&mut next);

        if next != current {
            let json = serde_json::to_string(&next).map_err(|e| format!("blob serialize: {e}"))?;
            if let Err(error) = self.write_blob_raw(json.as_bytes()) {
                let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                *guard = None;
                return Err(error);
            }
        }

        let readback = self.read_blob_raw().and_then(|raw| match raw {
            None => Ok(HashMap::new()),
            Some(bytes) => {
                let json =
                    String::from_utf8(bytes).map_err(|e| format!("blob read-back utf8: {e}"))?;
                serde_json::from_str::<HashMap<String, String>>(&json)
                    .map_err(|e| format!("blob read-back json: {e}"))
            }
        });
        match readback {
            Ok(readback) if readback == next => {
                let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                *guard = Some(next);
                Ok(())
            }
            Ok(_) => {
                let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                *guard = None;
                Err("managed keychain read-back mismatch".to_string())
            }
            Err(error) => {
                let mut guard = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                *guard = None;
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_removes_prior_session_material() {
        let mut current = HashMap::from([
            (
                "electric_desktop_session".to_string(),
                "old-session".to_string(),
            ),
            ("logout_pending".to_string(), "1".to_string()),
        ]);
        let replacement = HashMap::from([(
            "electric_desktop_session".to_string(),
            "new-session".to_string(),
        )]);

        replace_entries(&mut current, &replacement);

        assert_eq!(current, replacement);
        assert!(!current.keys().any(|key| key.contains("identity")));
    }

    #[test]
    fn checked_replacement_leaves_current_entries_unchanged_on_validation_error() {
        let mut current = HashMap::from([
            (
                "electric_desktop_session".to_string(),
                "current-session".to_string(),
            ),
            ("unexpected_secret".to_string(), "value".to_string()),
        ]);
        let original = current.clone();

        let result =
            replace_entries_checked(&mut current, |_| Err("unsupported material".to_string()));

        assert!(result.is_err());
        assert_eq!(current, original);
    }

    #[test]
    fn checked_replacement_normalizes_the_current_map() {
        let mut current = HashMap::from([
            (
                "electric_desktop_session".to_string(),
                "fresh-session".to_string(),
            ),
            (
                "identity:10000000-0000-4000-8000-000000000002".to_string(),
                "legacy-value".to_string(),
            ),
        ]);

        replace_entries_checked(&mut current, |fresh| {
            assert_eq!(
                fresh.get("electric_desktop_session").map(String::as_str),
                Some("fresh-session")
            );
            Ok(HashMap::from([(
                "electric_desktop_session".to_string(),
                "fresh-session".to_string(),
            )]))
        })
        .unwrap();

        assert_eq!(
            current,
            HashMap::from([(
                "electric_desktop_session".to_string(),
                "fresh-session".to_string(),
            )])
        );
    }
}
