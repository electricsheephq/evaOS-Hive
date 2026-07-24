# evaOS Teams 0.4.23-es.1 boundary audit

Status: pre-sign source gate for the internal canary

This audit applies to the exact Buzz commit that contains this file, stacked on
`electricsheephq/buzz` commit
`484889567373c518641d6240100652524ad697c6`. It uses:

- reviewed architecture head
  `ff11efa8ccc24486cb780544218cb0e62c26ec4e`;
- dashboard staging PR 707 head
  `fbcd83f877187858e0e4812aab3c9ed1ebbcec68`;
- evaos-golden staging PR 265 head
  `1eb2c56d6767f2a6449cf42a009a4c2c1c4ef1b3`.

The dashboard and Golden heads are open draft PRs. This document records source
controls and the remaining evidence gates. It is not a merge, signed-artifact,
deployed-relay, VM-runtime, installed-app, or customer-canary claim.

## Invariants

- Supabase is the only business authority.
- The Electric Sheep control identity is the sole managed relay owner; customer
  humans are members.
- Both the channel and author allowlists gate VM agents.
- Relay control-plane policy blocks native membership bypass.
- Hermes owns memory, model, provider, permission, and tool behavior.
- Existing Mac Access is reused.
- OpenClaw is untouched.

## Threat table

| Asset | Actor | Trust boundary | Reachable attack | Control | Test or evidence | Residual risk | Terminal disposition |
|---|---|---|---|---|---|---|---|
| Relay operator key and per-community control key | Dashboard service or compromised operator | Supabase Vault to relay reconciliation | Reuse one key for deployment and member control, leak it to a client, or let a customer owner mutate relay policy | Architecture requires separate operator and control identities; dashboard PR 707 keeps private material in server-side custody and returns only public entitlement data | Dashboard PR 707 head plus architecture sections 6 and 7 | No deployed Vault policy, rotation exercise, or redacted runtime trace exists yet | Escalated to the pre-canary external evidence gate; blocks a runtime claim, not this source gate |
| Human private identity and opaque Desktop session | Renderer, local user, or stolen webview state | Tauri backend and macOS Keychain | Read an `nsec` or raw Desktop session from renderer storage, import an upstream Buzz identity, or sign while access is stale | Managed secret store uses the product-specific Keychain service; session stays backend-only; recovery and expiry fail closed; managed workspace rejects imported private keys | Managed credential, recovery, signing, and workspace tests in `desktop/src-tauri` | Keychain behavior still needs installed-app proof on the signed candidate | Source control fixed and verified; installed-app proof deferred to issue 10/canary |
| Agent private key and Hermes provider credentials | Human client, another service user, or diagnostic tooling | Golden desired state to per-agent systemd service | Expose agent key/provider material to the client, environment logs, or sibling agents | Golden PR 265 stages distinct systemd credentials, isolated Hermes homes, outbound-only bridge services, and redacted diagnostics | Golden PR 265 head | The draft is unmerged and no rendered unit/effective environment from a canary VM has been captured | Escalated external runtime evidence gate; no VM claim |
| Supabase membership, bound public key, challenge, and outbox | Renderer or stale member | Broker API to app and reconciler | Select a customer/community/agent, replay a challenge, or treat client data as assignment authority | Dashboard broker derives account and assignment from the authenticated session, binds challenges, and emits server-owned reconciliation work; Buzz installs only the returned relay and public identity | Dashboard PR 707 source and Buzz managed entitlement tests | No deployed broker or live outbox evidence in this source gate | Source contract verified; deployment/runtime evidence deferred |
| Managed relay connection | Malicious renderer content or compromised UI state | Webview IPC to native WebSocket plugin | Connect a signed human identity to an arbitrary relay, credential-bearing URL, alternate path, query, or account; reuse an old socket after revoke or community switch | Managed CSP denies direct renderer networking; native WebSocket connect and send recheck current signing authorization and accept only the exact credential-free active entitlement origin; each connection is bound to the entitlement generation and stale inbound frames are dropped before renderer delivery | `native_websocket::tests::managed_websocket_requires_the_exact_authorized_credential_free_origin`; `native_websocket::tests::entitlement_generation_change_drops_inbound_before_delivery`; product-contract CSP test; `scripts/test-managed-boundary-contract.mjs` | Relay-side cluster disconnect must still be observed during live revocation; CSP compatibility needs installed-app smoke | P1 fixed now; runtime revocation and installed-app network capture remain external gates |
| Native workspace icon fetch | Malicious renderer | Webview IPC to native HTTP | Turn the app into an arbitrary GET or redirect-hop SSRF client | Managed command requires current authorization and the exact active credential-free relay; it uses the no-redirect client | `commands::workspace::managed_tests::managed_workspace_icon_requires_current_authorized_credential_free_relay`; existing no-redirect client test; boundary contract | Other native HTTP commands remain governed by their own fixed or authenticated origins; installed network capture is still required | Gate-relevant P2 fixed now; endpoint inventory retained in focused/static tests |
| Native repositories and local agent profile state | Malicious renderer or stale community record | Webview IPC to filesystem/native agent management | Probe arbitrary paths, rewrite `REPOS`, regenerate nest state, enable desktop-owned agent profiles, or race a stale renderer workspace call against a broker entitlement switch | Managed builds reject repository validation/overrides and agent-managed profiles; filesystem and profile-reconcile side effects are compiled out of `apply_workspace`; managed `apply_workspace` validates but never writes the broker-owned relay override | Managed workspace validation tests and boundary contract | Native Buzz keeps its existing repository behavior | Gate-relevant P2 fixed now; native path intentionally unchanged |
| Message and attachment content in renderer storage | Local process, later webview session, or shared evidence collector | React state to localStorage | Persist raw relay events, drafts, attachment URLs, or mentions after the session | Managed startup clears only content-bearing draft/snapshot prefixes before providers mount; managed drafts remain memory-only; snapshots never read or write; preference/read-state keys remain | `managedRendererPersistence.test.mjs`, `managedDraftPersistence.test.mjs`, `messageSnapshot.test.mjs`, managed E2E purge scenario | Content necessarily exists in renderer memory while displayed; the relay stores messages by product design | P1 fixed now; no claim that message content never exists in memory or at the relay |
| ACP prompts, thoughts, tool metadata/results, relay replies, and deep links | Agent, relay, provider, or crafted link sender | Protocol input to application logs | Copy raw transcript, tool names/results, provider errors, relay notices/frames, or sensitive URLs into logs | ACP and relay diagnostics emit fixed categories/counts only; parser errors no longer copy raw frames; deep-link failures omit the supplied URL | ACP sentinel tests, relay reply/parser sentinel tests, and boundary contract | OS/library crash diagnostics outside these explicit logs need signed-candidate observation | P1 fixed now; crash-report/runtime observation remains an external gate |
| Channel membership and agent trigger authority | Customer owner, sibling member/agent, or crafted Nostr client | Relay connection to room policy and bridge subscription | Use native membership events, owner status, a DM, or a sibling identity to trigger an agent without both grants | Managed community control identity is the only relay owner; relay control-plane policy rejects customer mutation; ACP author filtering runs before subscription matching and channel policy is explicit | Collaboration-policy tests, author/channel allowlist tests, and Golden PR 265 bridge policy source | The integrated relay/VM behavior is not deployed in this source gate | Source controls verified; live unauthorized DM/channel matrix deferred to the canary |
| ACP session continuity | Another room, human identity, runtime command, agent, or corrupt state | Relay event to persisted ACP session map | Resume a transcript across room, identity, runtime, agent, or account | Session keys include relay hash, agent public key, runtime hash, and channel ID; credential-bearing URLs/arguments do not enter filenames; corrupt records fail closed | `crates/buzz-acp/src/session_store.rs` tests and boundary contract | Initial canary uses one dedicated relay/community, so relay origin is the account/community scope | Accepted for the dedicated canary; shared-host multi-customer qualification remains issue 12 |
| Hermes model/provider/permission/memory authority | Desktop config, relay message, or bridge environment | Buzz bridge to `hermes acp` | Override Hermes provider, model, permissions, tools, memory, or setup from the client | Buzz launches the declared ACP command; Hermes discovers provider setup through ACP initialize/auth methods; managed client has no Hermes provider configuration authority; Golden desired state preserves isolated Hermes homes | Buzz Hermes runtime tests and Golden PR 265 source | Effective VM environment and real ACP launch still require redacted runtime proof | Source contract verified; runtime proof deferred |
| Managed upstream and auxiliary endpoints | Renderer, updater, Builderlab, support, push, or telemetry code | Managed product policy/CSP/native commands to network | Contact upstream Buzz/Builderlab/GitHub/update/support/push/telemetry services or probe arbitrary loopback HTTP services through crafted media | Managed product policy disables updater and upstream hosted services; CSP permits IPC, Tauri assets, inline/blob content, and the `buzz-media:` asset protocol but no general loopback image/media origin; focused E2E forbids named upstream hosts; native relay commands are tied to entitlement | Product-contract tests, managed endpoint E2E, and boundary contract | Browser request spying does not observe native sockets; external profile-image URLs are intentionally blocked and fall back to local/relay-proxied representations; installed network capture is still needed | Source gate fixed and verified; external-image restriction accepted for the canary; installed-app capture deferred |
| Community-scoped revocation and same-key behavior | Removed member or same key in another community | Broker revision/relay cluster/app entitlement | Keep an existing socket alive, reuse access across a community, or continue signing after expiry | Relay cluster disconnect, managed signing expiry/revoke, and tenant-bound broker contracts fail closed | Existing cluster disconnect, same-key/different-community, and signing expiry tests | End-to-end removal within 15 seconds needs deployed broker, relay, app, and VM evidence | Source controls verified; exact timing remains a canary gate |
| License notices and product identity | Packager or downstream distributor | Source/bundle configuration to artifact | Remove Apache notice, mislabel Buzz origin, or collide with upstream bundle identity | Managed product contract pins distinct name, bundle ID, scheme, artifact name, notice resources, and attribution | Product-contract and branding tests | No artifact exists yet | Source configuration verified; artifact inspection deferred to issue 10 |

## Finding dispositions

1. Unbounded ACP/relay content logging — **fixed now**.
2. Managed drafts and message snapshots in renderer persistence — **fixed now**.
3. Client-supplied relay/network authority through CSP/native WebSocket —
   **fixed now**.
4. Arbitrary managed workspace icon GET/redirect — **fixed now**.
5. Managed repository/profile filesystem mutation — **fixed now**.
6. Dedicated-relay session scope for the initial canary — **accepted
   tradeoff**; shared multi-customer relay qualification is issue 12.
7. Direct external profile-image loading — **accepted tradeoff** for the
   managed canary; local, inline, and relay-proxied media remain allowed.
8. Vault policy/rotation, rendered VM credentials, effective Hermes
   environment, installed-app network capture, and live revocation timing —
   **escalated external evidence gates**. They cannot be proven by this source
   diff and must not be represented as complete.

The first independent review of pushed head
`6242044bbeecf5809e5663ecf44f5cc12f521720` found four supported-path
blockers. Each has one terminal disposition in the revised candidate:

1. Residual raw ACP wire and relay-auth logging — **fixed now** with
   content-free wire metadata, relay error classes, and sentinel tests.
2. Managed `apply_workspace` relay-authority race — **fixed now** by making
   managed workspace application read-only with respect to the broker-owned
   relay override.
3. Cross-tenant inbound delivery after entitlement transition — **fixed now**
   by binding native sockets to the entitlement generation and dropping stale
   inbound frames before renderer delivery.
4. Arbitrary loopback image/media access in managed CSP — **fixed now** by
   removing the loopback wildcard and retaining only the `buzz-media:` asset
   protocol.

The delta-only review of
`6242044bbeecf5809e5663ecf44f5cc12f521720..2b00d9568e661cf0e3c6beddd5ac482fd005b7cd`
found two additional fix-lane blockers:

1. Raw ACP initialize-result logging — **fixed now** with content-free
   capability/auth-method/byte counts and a sentinel/static regression.
2. Refresh-driven socket invalidation and transition ordering — **fixed now**
   by preserving the generation for materially identical unexpired authority,
   incrementing it before any changed authority state, and emitting a
   content-free terminal callback when a socket becomes stale.

## Exact next gate

The source candidate may advance only after focused tests, canonical CI, and an
independent auth/tenant/secret/content review pass on its exact pushed head.
Signing remains blocked until issue 10 has explicit signing authority and the
external runtime evidence above is attached without secrets or customer
content.
