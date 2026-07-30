# Hive v0.5.2 Electric disposition matrix

Issue: [electricsheephq/evaOS-Hive#81](https://github.com/electricsheephq/evaOS-Hive/issues/81)

This matrix controls the reset from the current Electric merge head
`0421bc5bb949c917507a1808559a4353127e17d3` to exact upstream Buzz v0.5.2
`3e48f1b2365d326ee1c9582448d86a99b44ecd5d`.

The target is not a rebase of the existing Hive tree. It is a fresh upstream
base with only the Electric seams that upstream cannot provide.

## Patch-input rules

| Current lineage | Count | Disposition |
|---|---:|---|
| Historical Hive lineage `a13085e9ac9a7c8dbd9426a6b88fc75abf62220e..ce2850ca75a4d528baa4280953331b13188664a2` | 120 non-merge commits | **Drop as patch inputs.** These commits record superseded managed-fork implementations. Their surviving final file groups are dispositioned below; none may be cherry-picked wholesale. |
| Clean v0.5.1 adoption lineage `a13085e9ac9a7c8dbd9426a6b88fc75abf62220e..b75e26d298d014996d329044129618d21f580ab3` | 14 non-merge commits | **Disposition individually below.** These are the only candidate patch inputs, and mixed commits still require narrow replay. |
| Merge `0421bc5bb949c917507a1808559a4353127e17d3` | 1 merge commit | **Drop.** Never cherry-pick the merge; replay only approved seams from its second-parent lineage. |

The two non-merge ranges above cover all 134 Electric commits reachable from
the current merge head but absent from the v0.5.1 upstream base.

## Clean-lineage commit dispositions

| Commit | Subject | Disposition | Reason |
|---|---|---|---|
| `9c901b9ca6cd` | docs(hive): disposition Buzz v0.5.1 adoption | **Redesign** | Replace v0.5.1 adoption evidence with this v0.5.2 reset matrix and the issue #81 upstream dispositions. |
| `3d7faa2320bf` | feat(hive): add v0.5.1 product package seam | **Replay narrowly / redesign for v0.5.2** | Keep Hive bundle/product identity, icons, notices, support destination, and signed fork updater. Preserve the upstream `0.5.2` package versions and generate only the `0.5.2-es.1` Hive overlay. |
| `d29c3894bc67` | feat(hive): add managed OAuth community adapter | **Replay narrowly / redesign; no wholesale cherry-pick** | Keep Electric OAuth, active membership/seat checks, durable session entitlement, and server-selected company/community/relay. The old top-level gate makes native identity recovery unreachable, so preserve the v0.5.2 recovery flow and defer recoverable identity-envelope work to #78. |
| `347feb0af8c6` | feat(hive): project company VM agents | **Redesign** | Upstream relay agent records remain authoritative. Electric data may only authorize or narrowly decorate an already-registered public identity. Do not replay a second catalog or Hermes runtime authority. |
| `cac09e2b3af4` | chore(hive): preserve adoption size ratchet | **Redesign** | Recompute the ratchet from the v0.5.2 thin patch; do not preserve v0.5.1 file counts or ceilings. |
| `ea7902d2352c` | fix(desktop): bound relay metadata read-back | **Drop unless reproduced** | Native v0.5.2 behavior wins. Reintroduce only the smallest bounded native fix if an issue #81 supported path reproduces relay read-after-write failure. |
| `d03f9fcd718c` | fix(desktop): preserve profile updates across relay lag | **Drop unless reproduced** | v0.5.2 profile and avatar changes must be exercised first; no second profile authority. |
| `83dd0490544e` | Fix huddle discovery replay | **Drop unless reproduced** | Use native v0.5.2 huddles. Reintroduce only after a current supported-path reproduction. |
| `11fe0c72ef21` | fix(hive): close logout gate and restore governance | **Replay narrowly** | Preserve OAuth/session logout and dual-gate revocation. Drop native collaboration/runtime suppression and carry only current thin-adapter documentation. |
| `b8f8e7a0cb91` | ci(hive): fetch adoption base for desktop ratchet | **Redesign** | Any ratchet must target exact v0.5.2 and the new reset checker. |
| `5e40d07da6f7` | fix(hive): keep managed adapters clippy-clean | **Drop as a patch input** | Apply only compiler fixes required by the replayed v0.5.2 adapter and prove them with current Clippy. |
| `abeb6e7776a1` | test(hive): preserve auth state across identity restore failure | **Defer to #78** | Recovery/envelope behavior belongs to the serialized #78 client/backend contract after #81 freezes. |
| `8b42a7a57a61` | test(desktop): stabilize channel smoke assertions | **Drop unless reproduced** | Keep upstream channel tests unchanged unless the exact v0.5.2 managed canary exposes the same deterministic assertion problem. |
| `b75e26d298d0` | ci: honor fork push gateway image | **Replay only if still required** | Preserve fork-owned image publication without changing upstream local behavior; first compare the v0.5.2 workflow. |

## Final file-group dispositions

| File group from current Hive main | Disposition | v0.5.2 target |
|---|---|---|
| `desktop/src-tauri/hive/**`, `desktop/public/hive-icon.png`, `desktop/scripts/build-hive.mjs`, `desktop/src-tauri/tauri.hive.conf.json`, package/build metadata | **Replay narrowly** | Hive name, bundle ID, icons, notices, exact artifact naming, and signed Hive updater channel only. |
| `desktop/src-tauri/src/product_contract.rs`, `product_policy.rs`, `desktop/src/shared/product/**`, updater and feedback copy | **Replay narrowly** | Product identity and support/updater configuration. Product policy must not disable native collaboration, local runtimes, agent restore, or unrelated settings. |
| `desktop/src-tauri/src/evaos_teams.rs`, `device_code.rs`, `EvaosTeamsAuthGate.tsx`, `features/evaosTeams/api*`, `managedCommunity*` | **Replay narrowly / redesign** | OAuth, active company membership/seat entitlement, durable desktop session, and server-selected company/community/relay. No managed collaboration broker or Hermes authority. |
| `app_state.rs`, `app_state/signing.rs`, `secret_store.rs`, `secret_store/managed.rs`, identity commands | **Replay only the dual gate** | Native private key remains local and signs native events; managed authorization gates signing/relay access. OAuth-only recoverable identity restoration is #78, not #81. |
| `commands/workspace.rs`, `deep_link.rs`, community storage and community selection hooks | **Replay narrowly** | Reject client-selected managed authority and install the single server-selected company relay while leaving unmanaged Buzz unchanged. |
| `commands/channels.rs`, `commands/dms.rs`, `metadata_poll.rs`, `commands/profile.rs`, huddle state files, sidebar/channel UI, relay-session changes | **Drop unless reproduced** | Native v0.5.2 collaboration, profiles, huddles, sidebar, and relay behavior are authoritative. |
| `evaos_teams/company_directory.rs`, narrow `features/evaosTeams/api.ts`/`hooks.ts`, `companyAgentCatalog*`, `relayAgentsQuery.ts`, minimal agent view integration | **Redesign** | Intersect server-scoped authorized public keys with native relay records. Signed relay kind-0/kind-10100 data remains authoritative for profile, avatar, room membership, responder metadata, mentions, DMs, and presence. |
| Built-in starter presentation predicates and stable IDs | **Replay presentation only** | Hide `builtin:fizz`, `builtin:honey`, `builtin:bumble`, and `builtin-team:welcome` only after canonical registered `tars`, `samantha`, and `hal-9000` company identities exist. Preserve stored records, unmanaged Buzz, and explicit custom local agents. |
| `desktop/src-tauri/src/lib.rs` managed checks that suppress Mesh/local agent restore or native event flushing | **Drop broad guards** | Keep only the auth state registration and commands needed for OAuth/admission. Native agent/runtime lifecycle remains upstream-owned and capability-gated. |
| `commands/identity.rs` managed import/restore guards and the old top-level auth-gate recovery state | **Redesign** | Native v0.5.2 lost/keyring recovery must remain reachable. Managed logout revokes only the Electric session and never rotates or deletes the native identity. |
| Additive Electric settings | **Add the missing narrow seam** | Show company/community, server-selected relay, managed session, updater channel, and support without hiding or replacing native Settings pages. |
| `AGENTS.md`, `VISION_HIVE.md`, `docs/hive-thin-adapter-runbook.md` | **Replay current architecture text** | Keep the thin-adapter invariants and update exact source identities. Do not import historical implementation authority. |
| v0.5.1 disposition JSON/Markdown and checker | **Replace** | Use the issue #81 v0.5.2 upstream dispositions plus this Electric matrix; remove the obsolete v0.5.1 ratchet. |
| `.github/workflows/ci.yml`, `.github/workflows/docker.yml`, size-check scripts, e2e bridge/tests | **Redesign after code replay** | Preserve canonical fork CI/package proof for the exact v0.5.2 head without freezing old-base sizes or broad managed behavior. |
| Upstream `release.yml` / `signed-macos-canary.yml` | **Keep upstream unchanged; add a separate fork-only Hive release lane later** | The upstream jobs are guarded to `block/buzz`, so current Hive main has no fork-active signed release workflow. #81 source CI must remain unsigned; the later artifact gate needs an exact-tag Hive workflow bound to the package contract and updater signature. |

## Replay order

1. Product package, branding, updater, and support configuration.
2. OAuth, membership/seat entitlement, durable session, and server-selected
   company/community/relay admission while keeping native identity recovery
   reachable.
3. The smallest native company-agent authorization/presentation seam proven
   necessary on v0.5.2.
4. Thin-adapter tests and a recomputed v0.5.2 patch ceiling.
5. Only reproduced native-path compatibility deltas.

No Hermes discovery, readiness, authentication, model, launch, or session code
is a replay input. Exact v0.5.2 already owns Hermes through upstream BYOH and
the `buzz-acp -> hermes-acp` path.
