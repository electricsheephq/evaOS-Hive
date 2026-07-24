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

The final targeted review of
`2b00d9568e661cf0e3c6beddd5ac482fd005b7cd..4d2168ad2912388225d653e344f9c0f9d13c1da5`
found one remaining check-to-enqueue race. Its disposition is **fixed now**:
managed inbound generation validation and renderer enqueue share the same
entitlement-transition lock, with a deterministic race regression proving that
only a content-free `Close` is delivered after a transition.

## Upstream v0.4.24 candidate disposition

The managed candidate stays pinned to the v0.4.23 line. It was not rebased onto
v0.4.24. Every commit from upstream v0.4.23
`acfbb1bb6af54cb29cb152496ff43b8285dcb8cf` through v0.4.24
`710ed9fff57878a1d69f809b80a6ee0416c53fc4` received the following
candidate-scoped terminal disposition:

| Upstream commit | Change | Candidate-scoped disposition |
|---|---|---|
| `bcc3e1306946528102bb26be9a7c41299e2f8e00` | Configurable Redis pool size (#2521) | **Not applicable** to the source security/reconnect gate; relay capacity tuning remains an operator/runtime decision. |
| `03f645ef7e139f2eb703b477150034e8238062ae` | Benchmark script cleanup | **Not applicable**; no candidate product or canary behavior changes. |
| `06e3d82b04ab326a36694264ffb4b9dd94ec5661` | Optional harness onboarding skip (#2360) | **Accepted follow-up**; the managed login and Hermes launch contracts do not require this native onboarding UX change. |
| `d0ab3fdb054e0cfedbf21e4c5143ad6c671c10cc` | Strip channel-name hash prefixes (#2250) | **Accepted follow-up**; channel-name normalization does not affect the current trust or reconnect gate. |
| `1e68c6c05021ef2fc93ed0a02a84009ce94cdda5` | Community-rail drag reorder (#2549) | **Accepted follow-up**; unrelated product UX. |
| `55c9211241fcfb92a36ed5d6935cb4d2b3ae0702` | Populate team instructions in edit dialog (#2565) | **Accepted follow-up**; unrelated team-edit UX. |
| `8f8f5fa5a4b2463cdc6c2a527acb7086150cdaae` | Sanitize animated image uploads (#2524) | **Accepted follow-up**; it repairs animated-upload compatibility and metadata policy but is not required for the internal canary's managed authority, content-persistence, or reconnect boundary. The relay remains the final upload-policy authority. |
| `659e6a8d651b52602142f1e5b91817b2e0ca047f` | Protect the Sprig rolling tag (#2221) | **Not applicable**; the evaOS Teams candidate does not publish or move that tag. |
| `9cf9539040aee9774cf8d12b651348a2806c05c0` | Omit empty optional model control (#2262) | **Accepted follow-up**; Hermes remains model/provider authority and this UI refinement is not required for launch correctness. |
| `4253688bf05c21a066bc5a5dc928256163fd3451` | Add `just production` (#2572) | **Not applicable**; development recipe only. |
| `df0a0861776f358e8c365e18a429900044b12989` | Install rustls provider in Buzz CLI (#2590) | **Not applicable**; the managed desktop native WebSocket path, not `buzz-cli` WSS publishing, is in the candidate canary path. |
| `8cb05028be84829501a4f87f8db4ee7034fb786f` | Observer archive hydration/paging (#2574) | **Accepted follow-up**; unrelated observer UX and archive paging. |
| `80244f82318c85f931d1055e419456945c5eca99` | Avatar upload lifecycle fixes (#2277) | **Accepted follow-up**; unrelated profile UX. |
| `daeaf7c33d5415199a33cbc3dab00244fad5c219` | Channel lifecycle settings (#2427) | **Accepted follow-up**; the canary's managed channel/author policy is server-owned and does not depend on this UI. |
| `9ec52cfedf579d2ccb2021c216abd4c821a15165` | Retry failed initial relay dials (#2564) | **Fixed now** by exact backport as local commit `9defad62`; a rejected initial managed native dial must enter bounded reconnect handling. |
| `1911c69aa2912c1408bd6b21759b657458fb43af` | Send 1012 on graceful relay drain (#2575) | **Fixed now** by exact backport as local commit `c2104d74`; the VM-hosted relay restart path must notify every connected client and reject late registrations. |
| `6a56c8bdac6d115a0d6d48b24a2a04dc46b336c5` | Live relay kill/restart gate (#2583) | **Escalated to the live canary gate**; this test-only change requires an opted-in database, Redis, object store, relay process, and browser harness, and does not prove the managed Tauri entitlement boundary in canonical CI. Reuse its scenario when an authorized installed candidate and relay exist. |
| `cb42c8d5b60b15fd6ad47149c8785c7c863c8a37` | Restrict DM turns to owner/verified siblings (#2591) | **Fixed now; security-blocking** by exact backport as local commit `53bf5f97`. The candidate's agent bridge is reachable by member-authored DMs, so allowlisted humans cannot safely inherit turn authority through DM participation. Missing or unresolved channel metadata fails closed and setup mode uses the same gate. |
| `b096b0a15af4c4566365c5b1efe7f39b700222ed` | Preserve snapshot text chunks through sanitization (#2438) | **Accepted follow-up**; snapshot metadata preservation is unrelated to the canary trust/reconnect gate. |
| `f3f7688c3a4ecb0405ca8b26e0b6ee815e0f11e6` | Fast-track 1012 restart reconnects (#2579) | **Fixed now** by exact backport as local commit `b9b59ed9`; only an actual service-restart close resets accumulated delay, while entitlement invalidation keeps normal backoff. |
| `e67303f60334d6cd4224216080bd4b851fc5ee4d` | Build-gate upstream default-relay auto-connect (#2589) | **Not applicable** to the managed route; evaOS Teams installs broker-issued entitlement and its exact relay through the managed login flow, while upstream/default-relay auto-connect is disabled by product policy. Importing this separate release flag would create a second authority path. |
| `21573b6cb9695b46c11885cfb63bc548bbcd55de` | Mobile release process (#2144) | **Not applicable**; the milestone is a macOS internal canary and does not publish mobile artifacts. |
| `1e6e743a3570722f26878a3f2660371d8c0425ec` | Update `SECURITY.md` | **Accepted follow-up**; documentation-only and no candidate behavior changes. |
| `cca16635d69dc8bea5406013095d25f3b0e287d3` | Windows PATH and `.cmd` handling (#2563) | **Not applicable**; the internal canary target is macOS. |
| `5afa16157a63c71f2cd8a80aa7276de28ce1c54c` | Windows console/WSL alias fixes (#2587) | **Not applicable**; the internal canary target is macOS. |
| `95478919fc24bf5b43e29ce2c7e52a4c9c9287fc` | Per-community navigation memory (#2629) | **Accepted follow-up**; unrelated navigation UX and persistence deliberately remains outside this boundary patch. |
| `710ed9fff57878a1d69f809b80a6ee0416c53fc4` | v0.4.24 release version (#2627) | **Not applicable**; importing the version commit would falsely relabel the selectively backported `0.4.23-es.1` candidate as full v0.4.24. |

The selected patch set is dependency-closed for the current gate:
`#2564 + #2575 + #2579` supplies the production restart/reconnect behavior, and
the complete `#2591` patch supplies DM authorization, lazy channel-type
resolution, fail-closed unknown handling, and setup-mode parity. No unrelated
v0.4.24 ancestry is included. Author-allowlist revocations are still
process-snapshot state in #2591; restart/reload and the live unauthorized
DM/channel matrix remain explicit canary evidence gates.

## Exact next gate

The source candidate may advance only after focused tests, canonical CI, and an
independent auth/tenant/secret/content review pass on its exact pushed head.
Signing remains blocked until issue 10 has explicit signing authority and the
external runtime evidence above is attached without secrets or customer
content.
