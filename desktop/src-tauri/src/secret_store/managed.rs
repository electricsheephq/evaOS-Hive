use std::collections::HashMap;

use super::SecretStore;

fn replace_entries(current: &mut HashMap<String, String>, replacement: &HashMap<String, String>) {
    current.clear();
    current.extend(replacement.clone());
}

impl SecretStore {
    /// Atomically replace the managed keychain blob with exactly `entries`.
    pub fn replace_all(&self, entries: &HashMap<String, String>) -> Result<(), String> {
        self.mutate_blob(|map| replace_entries(map, entries))
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
}
