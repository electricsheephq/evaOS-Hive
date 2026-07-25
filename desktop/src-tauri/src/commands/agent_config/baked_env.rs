/// Return the key names of all non-empty baked build env vars.
///
/// Internal builds can bake provider credentials and other environment pairs
/// into the binary via `BUZZ_BUILD_AGENT_ENV`. Only key names cross this IPC
/// boundary. Managed Hive builds do not expose native agent settings.
#[tauri::command]
pub fn get_baked_build_env_keys() -> Vec<String> {
    if cfg!(feature = "evaos-teams-managed") {
        return Vec::new();
    }
    crate::managed_agents::baked_build_env()
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, _)| key)
        .collect()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BakedEnvEntry {
    pub key: String,
    /// The display value, with non-allowlisted values replaced by a mask.
    pub value: String,
    /// Whether `value` was masked before it crossed the Tauri boundary.
    pub masked: bool,
}

/// Return whether a baked environment value is explicitly safe to show.
///
/// The allowlist is case-insensitive and default-deny.
pub(super) fn is_safe_to_reveal(key: &str) -> bool {
    const SAFE_KEYS: &[&str] = &[
        "BUZZ_AGENT_PROVIDER",
        "BUZZ_AGENT_MODEL",
        "BUZZ_AGENT_THINKING_EFFORT",
        "DATABRICKS_HOST",
        "DATABRICKS_MODEL",
    ];
    let upper = key.to_ascii_uppercase();
    SAFE_KEYS.iter().any(|safe| upper == *safe)
}

/// Return baked agent environment with secret values masked.
///
/// Managed Hive builds return no native agent configuration.
#[tauri::command]
pub fn get_baked_build_env() -> Vec<BakedEnvEntry> {
    if cfg!(feature = "evaos-teams-managed") {
        return Vec::new();
    }
    crate::managed_agents::baked_build_env()
        .into_iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(key, value)| {
            let masked = !is_safe_to_reveal(&key);
            BakedEnvEntry {
                key,
                value: if masked {
                    "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}".to_string()
                } else {
                    value
                },
                masked,
            }
        })
        .collect()
}
