use std::collections::HashMap;

use super::SecretStore;

fn replace_entries(current: &mut HashMap<String, String>, replacement: &HashMap<String, String>) {
    current.clear();
    current.extend(replacement.clone());
}

fn store_entry_if_absent(
    current: &mut HashMap<String, String>,
    key: &str,
    candidate: &str,
) -> String {
    current
        .entry(key.to_string())
        .or_insert_with(|| candidate.to_string())
        .clone()
}

impl SecretStore {
    /// Atomically replace the managed keychain blob with exactly `entries`.
    pub fn replace_all(&self, entries: &HashMap<String, String>) -> Result<(), String> {
        self.mutate_blob(|map| replace_entries(map, entries))
    }

    /// Atomically retain an existing value or store `candidate` for `key`.
    ///
    /// The mutation reads the durable blob while holding the cross-process
    /// lock, so a second Hive process cannot overwrite an already-staged key
    /// or lose unrelated entries through a stale in-memory snapshot.
    pub fn store_if_absent(&self, key: &str, candidate: &str) -> Result<String, String> {
        let mut stored = None;
        self.mutate_blob(|map| {
            stored = Some(store_entry_if_absent(map, key, candidate));
        })?;
        stored.ok_or_else(|| "keychain mutation did not return a stored value".to_string())
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
    fn store_if_absent_preserves_existing_value_and_unrelated_entries() {
        let mut current = HashMap::from([
            ("staged".to_string(), "first".to_string()),
            ("unrelated".to_string(), "keep".to_string()),
        ]);
        let stored = store_entry_if_absent(&mut current, "staged", "second");
        assert_eq!(stored, "first");
        assert_eq!(current.get("staged").map(String::as_str), Some("first"));
        assert_eq!(current.get("unrelated").map(String::as_str), Some("keep"));
    }
}
