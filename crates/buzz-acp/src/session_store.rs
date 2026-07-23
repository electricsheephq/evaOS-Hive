use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const STORE_VERSION: u32 = 1;
const MAX_STORE_BYTES: u64 = 1024 * 1024;
const MAX_MAPPINGS: usize = 10_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SessionScope {
    relay_hash: String,
    agent_pubkey: String,
    runtime_hash: String,
}

impl SessionScope {
    pub fn new(
        relay_url: &str,
        agent_pubkey: &str,
        agent_command: &str,
        agent_args: &[String],
    ) -> Self {
        let runtime = serde_json::to_vec(&(agent_command, agent_args))
            .expect("serializing strings cannot fail");
        Self {
            relay_hash: hash_bytes(relay_url.as_bytes()),
            agent_pubkey: agent_pubkey.to_ascii_lowercase(),
            runtime_hash: hash_bytes(&runtime),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SessionMapping {
    scope: SessionScope,
    channel_id: Uuid,
    session_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PersistedState {
    version: u32,
    mappings: Vec<SessionMapping>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            mappings: Vec::new(),
        }
    }
}

/// Durable channel-to-ACP-session mappings.
///
/// The store contains identifiers only. Relay and runtime values are hashed so
/// credential-bearing URLs and command arguments never reach disk.
pub struct SessionStore {
    path: PathBuf,
    scope: SessionScope,
    state: Mutex<PersistedState>,
}

impl SessionStore {
    pub fn open(path: PathBuf, scope: SessionScope) -> Self {
        let state = read_state(&path).unwrap_or_else(|error| {
            tracing::warn!(
                target: "session_store",
                path = %path.display(),
                "ignoring invalid ACP session mapping store: {error}"
            );
            PersistedState::default()
        });
        Self {
            path,
            scope,
            state: Mutex::new(state),
        }
    }

    pub fn get(&self, channel_id: Uuid) -> Option<String> {
        self.state
            .lock()
            .expect("session store mutex poisoned")
            .mappings
            .iter()
            .find(|mapping| mapping.scope == self.scope && mapping.channel_id == channel_id)
            .map(|mapping| mapping.session_id.clone())
    }

    pub fn record(&self, channel_id: Uuid, session_id: String) -> std::io::Result<()> {
        let mut state = self.state.lock().expect("session store mutex poisoned");
        let mut next = state.clone();
        if let Some(mapping) = next
            .mappings
            .iter_mut()
            .find(|mapping| mapping.scope == self.scope && mapping.channel_id == channel_id)
        {
            mapping.session_id = session_id;
        } else {
            if next.mappings.len() >= MAX_MAPPINGS {
                return Err(std::io::Error::other(
                    "ACP session mapping store reached its entry limit",
                ));
            }
            next.mappings.push(SessionMapping {
                scope: self.scope.clone(),
                channel_id,
                session_id,
            });
        }
        sort_mappings(&mut next.mappings);
        write_state(&self.path, &next)?;
        *state = next;
        Ok(())
    }

    pub fn remove(&self, channel_id: Uuid) -> std::io::Result<bool> {
        let mut state = self.state.lock().expect("session store mutex poisoned");
        let mut next = state.clone();
        let old_len = next.mappings.len();
        next.mappings
            .retain(|mapping| !(mapping.scope == self.scope && mapping.channel_id == channel_id));
        if next.mappings.len() == old_len {
            return Ok(false);
        }
        write_state(&self.path, &next)?;
        *state = next;
        Ok(true)
    }

    #[cfg(test)]
    pub fn test_path(&self) -> &Path {
        &self.path
    }
}

fn hash_bytes(value: &[u8]) -> String {
    hex::encode(Sha256::digest(value))
}

fn sort_mappings(mappings: &mut [SessionMapping]) {
    mappings.sort_by(|left, right| {
        (
            &left.scope.relay_hash,
            &left.scope.agent_pubkey,
            &left.scope.runtime_hash,
            left.channel_id,
        )
            .cmp(&(
                &right.scope.relay_hash,
                &right.scope.agent_pubkey,
                &right.scope.runtime_hash,
                right.channel_id,
            ))
    });
}

fn read_state(path: &Path) -> Result<PersistedState, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PersistedState::default());
        }
        Err(error) => return Err(error.to_string()),
    };
    if metadata.len() > MAX_STORE_BYTES {
        return Err(format!(
            "file is {} bytes; limit is {MAX_STORE_BYTES}",
            metadata.len()
        ));
    }
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let state: PersistedState =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if state.version != STORE_VERSION {
        return Err(format!("unsupported version {}", state.version));
    }
    if state.mappings.len() > MAX_MAPPINGS {
        return Err(format!(
            "store has {} mappings; limit is {MAX_MAPPINGS}",
            state.mappings.len()
        ));
    }
    Ok(state)
}

fn write_state(path: &Path, state: &PersistedState) -> std::io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(std::io::Error::other)?;
    let mut file = AtomicWriteFile::open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(&bytes)?;
    file.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("buzz-acp-{name}-{}.json", Uuid::new_v4()))
    }

    fn scope(relay: &str, key: &str, command: &str) -> SessionScope {
        SessionScope::new(relay, key, command, &["acp".to_owned()])
    }

    #[test]
    fn round_trip_survives_reopen_and_keeps_channels_isolated() {
        let path = temp_path("round-trip");
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let scope = scope("wss://relay.example", "aabb", "hermes");
        let store = SessionStore::open(path.clone(), scope.clone());
        store.record(first, "session-one".into()).unwrap();
        store.record(second, "session-two".into()).unwrap();
        drop(store);

        let reopened = SessionStore::open(path.clone(), scope);
        assert_eq!(reopened.get(first).as_deref(), Some("session-one"));
        assert_eq!(reopened.get(second).as_deref(), Some("session-two"));
        assert!(reopened.remove(first).unwrap());
        assert_eq!(reopened.get(first), None);
        assert_eq!(reopened.get(second).as_deref(), Some("session-two"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn scope_changes_do_not_resume_old_mappings() {
        let path = temp_path("scope");
        let channel = Uuid::new_v4();
        let original =
            SessionStore::open(path.clone(), scope("wss://relay.example", "aabb", "hermes"));
        original.record(channel, "session-one".into()).unwrap();
        drop(original);

        for changed in [
            scope("wss://other.example", "aabb", "hermes"),
            scope("wss://relay.example", "ccdd", "hermes"),
            scope("wss://relay.example", "aabb", "hermes-acp"),
        ] {
            assert_eq!(SessionStore::open(path.clone(), changed).get(channel), None);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupt_state_fails_closed_and_next_record_repairs_it() {
        let path = temp_path("corrupt");
        std::fs::write(&path, b"{not-json").unwrap();
        let channel = Uuid::new_v4();
        let store =
            SessionStore::open(path.clone(), scope("wss://relay.example", "aabb", "hermes"));
        assert_eq!(store.get(channel), None);
        store.record(channel, "replacement".into()).unwrap();
        drop(store);
        assert_eq!(
            SessionStore::open(path.clone(), scope("wss://relay.example", "aabb", "hermes"))
                .get(channel)
                .as_deref(),
            Some("replacement")
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn concurrent_records_preserve_every_channel() {
        let path = temp_path("concurrent");
        let store = Arc::new(SessionStore::open(
            path.clone(),
            scope("wss://relay.example", "aabb", "hermes"),
        ));
        let channels: Vec<_> = (0..16).map(|_| Uuid::new_v4()).collect();
        let threads: Vec<_> = channels
            .iter()
            .enumerate()
            .map(|(index, channel)| {
                let store = Arc::clone(&store);
                let channel = *channel;
                std::thread::spawn(move || {
                    store.record(channel, format!("session-{index}")).unwrap();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        for (index, channel) in channels.iter().enumerate() {
            assert_eq!(
                store.get(*channel),
                Some(format!("session-{index}")),
                "mapping {index} was lost"
            );
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn serialized_state_excludes_raw_relay_and_runtime_values() {
        let path = temp_path("redaction");
        let channel = Uuid::new_v4();
        let store = SessionStore::open(
            path.clone(),
            SessionScope::new(
                "wss://secret.example?token=credential-marker",
                "aabb",
                "/private/runtime-marker/hermes",
                &["--token=argument-marker".into()],
            ),
        );
        store.record(channel, "session-id".into()).unwrap();
        let serialized = std::fs::read_to_string(&path).unwrap();
        assert!(!serialized.contains("credential-marker"));
        assert!(!serialized.contains("runtime-marker"));
        assert!(!serialized.contains("argument-marker"));
        assert!(serialized.contains("session-id"));
        let _ = std::fs::remove_file(path);
    }
}
