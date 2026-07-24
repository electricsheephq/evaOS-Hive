import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const read = (relativePath) =>
  readFileSync(path.join(repoRoot, relativePath), "utf8");

const bootstrap = read("desktop/src/main.tsx");
const persistence = read(
  "desktop/src/shared/product/managedRendererPersistence.ts",
);
const drafts = read("desktop/src/features/messages/lib/useDrafts.ts");
const snapshots = read("desktop/src/features/messages/lib/messageSnapshot.ts");
const acp = read("crates/buzz-acp/src/acp.rs");
const pool = read("crates/buzz-acp/src/pool.rs");
const relay = read("crates/buzz-acp/src/relay.rs");
const deepLinks = read("desktop/src-tauri/src/deep_link.rs");
const workspaceCommands = read("desktop/src-tauri/src/commands/workspace.rs");
const nativeWebsocket = read("desktop/src-tauri/src/native_websocket.rs");
const productContract = read("desktop/src-tauri/src/product_contract.rs");
const managedTauriConfig = JSON.parse(
  read("desktop/src-tauri/tauri.evaos-teams.conf.json"),
);
const managedE2e = read("desktop/tests/e2e/evaos-teams-branding.spec.ts");
const sessionStore = read("crates/buzz-acp/src/session_store.rs");
const collaborationPolicy = read(
  "crates/buzz-relay/src/handlers/collaboration_policy.rs",
);

for (const prefix of [
  "buzz-drafts.v1",
  "buzz-drafts.v2",
  "buzz-channel-messages.v1",
]) {
  assert.match(
    persistence,
    new RegExp(prefix.replaceAll(".", "\\.")),
    `${prefix} must remain in the managed sensitive-storage purge`,
  );
}
assert.match(bootstrap, /clearManagedSensitiveRendererState\(\)/);
assert.match(
  bootstrap,
  /if \(!desktopProductPolicy\(\)\.managed\) \{\s*await migrateLegacyCommunityStorageBeforeRender\(\)/,
);
assert.match(drafts, /rendererContentPersistenceAllowed\(\)/);
assert.match(snapshots, /rendererContentPersistenceAllowed\(\)/);

for (const [source, forbidden, description] of [
  [acp, /target: "acp::stream", "\{text\}"/, "agent message body logging"],
  [acp, /target: "acp::thought", "\{text\}"/, "agent thought logging"],
  [acp, /tool_call: \{title\}/, "tool title logging"],
  [
    acp,
    /target: "acp::wire"[^;]*to_string\(&msg\)/s,
    "raw outbound ACP wire logging",
  ],
  [acp, /target: "acp::wire"[^;]*\{trimmed\}/s, "raw inbound ACP wire logging"],
  [acp, /"line": trimmed/, "raw ACP parse-error observation"],
  [acp, /initialize response: \{result\}/, "raw ACP initialize logging"],
  [pool, /session_prompt error: \{e\}/, "raw ACP error logging"],
  [relay, /raw: \{text\}/, "raw relay frame logging"],
  [relay, /relay NOTICE: \{message\}/, "raw relay NOTICE logging"],
  [relay, /message=\{message\}/, "raw relay acknowledgement logging"],
  [deepLinks, /eprintln!\([^;]*\{url_str\}/s, "raw deep-link logging"],
]) {
  assert.doesNotMatch(source, forbidden, description);
}
assert.match(acp, /pub\(crate\) fn log_class/);
assert.match(acp, /fn wire_log_metadata/);
assert.match(acp, /fn initialize_log_metadata/);
assert.match(relay, /fn relay_reply_class/);
assert.match(relay, /fn log_class\(&self\)/);
assert.match(workspaceCommands, /validate_managed_workspace_icon_request/);
assert.match(workspaceCommands, /&state\.media_fetch_client/);
assert.match(
  workspaceCommands,
  /Managed workspaces cannot override the repositories directory/,
);
assert.match(
  workspaceCommands,
  /#\[cfg\(not\(feature = "evaos-teams-managed"\)\)\]\s*\{\s*if let Some\(nest\)/,
);
assert.match(
  workspaceCommands,
  /#\[cfg\(not\(feature = "evaos-teams-managed"\)\)\]\s*\{\s*let mut override_guard = state\.relay_url_override/s,
);

assert.match(nativeWebsocket, /validate_managed_websocket_request/);
assert.match(nativeWebsocket, /state\.signing_keys\(\)\?/);
assert.match(
  nativeWebsocket,
  /Managed relay must match the active entitlement/,
);
assert.match(
  nativeWebsocket,
  /let connection_url = manager\.connection_url\(id\)\.await\?/,
);
assert.match(nativeWebsocket, /struct ManagedConnectionAuthority/);
assert.match(
  nativeWebsocket,
  /is_some_and\(\|authority\| !authority\.is_current\(\)\)/,
);
assert.match(
  nativeWebsocket,
  /serde_json::to_value\(OutboundMessage::Close\(None\)\)/,
);
assert.match(
  read("desktop/src-tauri/src/app_state/signing.rs"),
  /fetch_add\(1, Ordering::AcqRel\)[\s\S]*evaos_teams_authorized\.store\(false, Ordering::Release\)/,
);
assert.match(productContract, /EVAOS_TEAMS_CSP/);
assert.match(productContract, /"csp": EVAOS_TEAMS_CSP/);
const managedCsp = managedTauriConfig.app?.security?.csp;
assert.equal(typeof managedCsp, "string");
assert.match(managedCsp, /default-src 'self'/);
assert.match(managedCsp, /connect-src ipc: http:\/\/ipc\.localhost/);
assert.doesNotMatch(managedCsp, /connect-src[^;]*(?:\*|https:|wss:|ws:)/);
assert.doesNotMatch(
  managedCsp,
  /img-src[^;]*http:\/\/(?:127\.0\.0\.1|localhost)(?=[:/])/,
);
assert.doesNotMatch(
  managedCsp,
  /media-src[^;]*http:\/\/(?:127\.0\.0\.1|localhost)(?=[:/])/,
);
assert.match(managedCsp, /media-src[^;]*buzz-media:/);

for (const host of [
  "app.builderlab.xyz",
  "communities.buzz.xyz",
  "pairing.buzz.xyz",
  "push.buzz.xyz",
  "support.buzz.xyz",
  "telemetry.buzz.xyz",
  "github.com",
  "raw.githubusercontent.com",
  "block.github.io",
]) {
  assert.match(
    managedE2e,
    new RegExp(host.replaceAll(".", "\\.")),
    `${host} must remain covered by the managed no-upstream network spy`,
  );
}

for (const field of [
  "relay_hash",
  "agent_pubkey",
  "runtime_hash",
  "channel_id",
]) {
  assert.match(
    sessionStore,
    new RegExp(`\\b${field}\\b`),
    `ACP session scope must retain ${field}`,
  );
}
assert.match(
  sessionStore,
  /credential-bearing URLs and command arguments never reach disk/,
);
assert.match(collaborationPolicy, /CollaborationPolicy::ControlPlane/);
assert.match(
  collaborationPolicy,
  /collaboration mutation requires the current community control identity/,
);

console.log(
  "managed identity, tenant, secret, and content boundary contract passed",
);
