use std::cell::RefCell;
use std::collections::HashMap;

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
    pub fn replace_all_checked<F>(&self, normalize: F) -> Result<(), String>
    where
        F: FnOnce(&HashMap<String, String>) -> Result<HashMap<String, String>, String>,
    {
        let outcome = RefCell::new(None);
        self.mutate_blob(|current| {
            let result = replace_entries_checked(current, normalize);
            *outcome.borrow_mut() = Some(result);
        })?;
        outcome
            .into_inner()
            .unwrap_or_else(|| Err("managed keychain normalization did not run".to_string()))
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
