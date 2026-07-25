# evaOS Teams 0.4.23-es.1 dependency dispositions

This record covers the Electric-only internal-canary integration candidate for
Buzz #9. It is not a claim about the current `block/buzz` release, a signed
artifact, deployment, or runtime.

## Candidate lineage

- Pinned upstream release ancestor:
  `acfbb1bb6af54cb29cb152496ff43b8285dcb8cf` (`v0.4.23`).
- Electric managed baseline:
  `23c6cb4c745667c4aaf117cbef519368ff856a34` (Buzz #8).
- Integrated feature baseline before dependency changes:
  `e3d707f6d65558959833b82d04f93a08abe2a574`.
- Current upstream main at integration time:
  `9cc9652c7dec9145b0bf0ce2c4b46c8191d215f8`. It is intentionally **not**
  an ancestor of this candidate.
- The exact final candidate head, canonical CI run, and independent review are
  head-sensitive evidence and are recorded on Buzz #9 and its staging PR.

Reviewed generic commits were transplanted with `git cherry-pick -x`; their
original branch bases were not merged:

| Issue | Reviewed source commit | Release-stack commit |
| --- | --- | --- |
| #2 | `a3984c1e5c0ad415d1f2a57fa81c1357c3c35c08` | `c2389be443b2e2043191dd05116df346cd8f71df` |
| #2 | `be38edde4d31a31f5a298b4e683cc1fa3987ff38` | `82dfd919113c0d72b0d9b792cf147707b9d6ad64` |
| #3 | `d2f92ac10d436f721408137337fe09af405c28b9` | `3e0b88eb08901337787aa6dbbfc542e84927e7ba` |
| #3 | `38092a555729ddc5f4e7508b5baa0c140ab3d061` | `0ca29007decb9c9ea8825c2ba0f4f01271dff8e8` |
| #3 | `147a0375829acc8a72f91572c752494f539efbfe` | `92e9d7c414794fc1b90e7aba25f317cde87f7569` |
| #6 | `62e8baab7233f7f6371069e6ee5901ce0073dd78` | `5948ccb4af3115a75bc1b8d2bd87441de69b532f` |
| #6 | `120ab9cad46ace2fbd307acf36b3f11b29046f16` | `798a54242919e7b97e31c3a07a031488fd79c46f` |
| #6 | `0a2ba218f4175124af77b9e523e14a6c6ff8a7df` | `68dd064f183daa02fd3f69aae7bae255ee2eccdc` |
| #15 | `f23538c93c000d67038148ce47a5ad374c6cbdbc` | `1f33d8c362233f70768e1548d59a765b1ecede6c` |
| #15 | `5ded65617d45e3b5c99e7e84e819914e9a0bec95` | `36b732ceb8a37dec816981ac432cad2fc3ebad17` |
| #15 | `758e8691dc9da4545ffbc2ee75a24757786c953d` | `5ba0c8c95f0d2fbdd707216e3d1168c79d48ef78` |
| #7 | `1fde7c483f050f37b5be610d8233b45fc1c909d1` | `e3d707f6d65558959833b82d04f93a08abe2a574` |

## Alert snapshot

At `2026-07-24T13:02:19Z`, GitHub showed ten open Dependabot alerts for the
fork's default branch at
`06e3d82b04ab326a36694264ffb4b9dd94ec5661`: two high, five moderate, and
three low. That default-branch view does not scan this stacked candidate, so
the target-specific candidate scans below determine release reachability.

| Alert | Severity | Candidate reachability | Terminal disposition |
| --- | --- | --- | --- |
| #9 `linkify-it` [GHSA-22p9-wv53-3rq4](https://github.com/advisories/GHSA-22p9-wv53-3rq4) | High | Production desktop composer path: `tiptap-markdown -> markdown-it -> linkify-it`. | **Fixed now.** Explicitly pinned and resolved to `5.0.2`, which covers both open `linkify-it` advisories. |
| #10 `linkify-it` [GHSA-v245-v573-v5vm](https://github.com/advisories/GHSA-v245-v573-v5vm) | High | Same production composer path. | **Fixed now.** `5.0.2`; no affected `5.0.0` or `5.0.1` lock entry remains. |
| #4 `glib` [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g) | Moderate | `glib 0.18.5` is reachable only through Tauri's GTK/WebKit Linux desktop target. `cargo tree --target aarch64-apple-darwin` does not select it. The canary desktop target is arm64 macOS; the Linux relay does not depend on GTK. | **Not applicable.** Outside both supported canary targets. A later Linux desktop release must re-disposition it. |
| #6 `opentelemetry_sdk` [GHSA-w9wp-h8wv-79jx](https://github.com/advisories/GHSA-w9wp-h8wv-79jx), desktop lock | Moderate | The signed workflow's `mesh-llm` feature selects `0.31.0`. The advisory affects inbound baggage-header extraction. Pinned Mesh `v0.73.1` uses only `Resource` and metrics exporter APIs; its source has no `BaggagePropagator` or `extract_with_context` call. | **Accepted tradeoff.** The affected function is not exercised on the supported path. Moving Mesh across its `0.31` constraint requires a Mesh source/tag change, not a lock-only patch. |
| #2 `opentelemetry_sdk` [GHSA-w9wp-h8wv-79jx](https://github.com/advisories/GHSA-w9wp-h8wv-79jx), root lock | Moderate | `0.31.0` is selected only under `mesh-llm` dev-dependencies of `buzz-relay`; it is absent from the production relay edge set. | **Not applicable.** Test/development-only, not shipped in the Linux relay or signed app. |
| #3 `cmov` [GHSA-3rjw-m598-pq24](https://github.com/advisories/GHSA-3rjw-m598-pq24) | Moderate | Production Linux relay path through `sqlx-postgres`; also used by signed-client crypto dependencies. | **Fixed now.** Root release lock updated from `0.5.3` to `0.5.4`; the desktop graph already selected `0.5.4`. |
| #8 `markdown-it` [GHSA-6v5v-wf23-fmfq](https://github.com/advisories/GHSA-6v5v-wf23-fmfq) | Moderate | Production desktop composer path through `tiptap-markdown`. | **Fixed now.** Explicitly pinned and resolved to `14.2.0`. |
| #5 `rpassword` [GHSA-2p6r-x3vv-xqm2](https://github.com/advisories/GHSA-2p6r-x3vv-xqm2), desktop lock | Low | The signed workflow's `mesh-llm` feature selects `5.0.1`. Pinned Mesh calls `prompt_password_stderr` only when both stdin and stderr are terminals; the supported signed GUI launch is non-terminal and instead uses environment/Keychain handling or returns `MissingPassphrase`. | **Accepted tradeoff.** The interrupted-terminal echo behavior is not reachable in the supported signed-app flow. Moving Mesh from `rpassword 5` to `7.5` requires an upstream source change. |
| #1 `rpassword` [GHSA-2p6r-x3vv-xqm2](https://github.com/advisories/GHSA-2p6r-x3vv-xqm2), root lock | Low | `5.0.1` is selected only by the `mesh-llm` dev-dependency of `buzz-relay`. | **Not applicable.** Test/development-only and not shipped. A major transitive upgrade would add risk without reducing canary exposure. |
| #7 `@babel/core` [GHSA-4x5r-pxfx-6jf8](https://github.com/advisories/GHSA-4x5r-pxfx-6jf8) | Low | Build-time only through the TanStack router plugin; excluded from `pnpm audit --prod`. | **Fixed now.** The compatible patch `7.29.6` is explicitly pinned so local and CI builds do not use the affected release. |

No open high-severity vulnerability remains in a supported canary path. No
realistic supported-path moderate vulnerable behavior remains unfixed.

## Candidate-only scan findings

These are not among the ten GitHub Dependabot alerts, but the candidate scans
gave each one a terminal disposition:

- Root Linux `cargo-deny check advisories` passes. It reports yanked
  `spin 0.9.8` through dev-only `mesh-llm -> mdns-sd -> flume`, and yanked
  `spin 0.10.0` through production `iroh -> n0-future -> futures-buffered`.
  Neither is a RustSec vulnerability advisory. **Accepted tradeoff** for this
  internal canary: both are upstream transitive selections and changing the
  networking stack is outside this alert-remediation packet.
- The arm64 macOS desktop scan with the signed workflow's `mesh-llm` feature
  reports seven informational `unmaintained` advisories, not vulnerability
  advisories:
  `RUSTSEC-2026-0150` (`audiopus_sys 0.2.2` through direct `opus`),
  `RUSTSEC-2020-0168` (`mach 0.1.2/0.3.2` through direct `user-idle`), and
  `RUSTSEC-2025-0075`, `RUSTSEC-2025-0080`, `RUSTSEC-2025-0081`,
  `RUSTSEC-2025-0098`, `RUSTSEC-2025-0100` (the `unic 0.9` family through
  Tauri's `urlpattern`). **Accepted tradeoff** for the internal canary:
  replacing the audio, idle-detection, or Tauri framework paths is a separate
  modernization decision, and the scan identifies no known vulnerability.
- Root Linux and arm64 macOS `cargo-deny check licenses` both pass. The
  unmatched allow-list entries are configuration warnings, not dependency
  license failures.
- `pnpm licenses list --prod --json` completes successfully. The dependency
  updates introduce no new license family beyond the candidate's existing
  production inventory.

## Focused proof

- `node scripts/test-canary-dependency-contract.mjs`
- `desktop/src/shared/lib/dependencySecurity.test.mjs`: the production
  `tiptap-markdown` dependency chain completes the published fuzzy-email,
  repeated-`mailto:`, and smartquote denial-of-service payload families within
  a generous one-second regression bound.
- `pnpm audit --prod --json`: zero vulnerabilities across 446 production and
  optional dependencies.
- `pnpm audit --json`: zero vulnerabilities across all 641 dependencies,
  including development dependencies.
- `cargo-deny --locked --target x86_64-unknown-linux-gnu check advisories`
- `cargo-deny --locked --target x86_64-unknown-linux-gnu check licenses`
- `cargo-deny --locked --target aarch64-apple-darwin --manifest-path
  desktop/src-tauri/Cargo.toml --features mesh-llm check advisories`
- `cargo-deny --locked --target aarch64-apple-darwin --manifest-path
  desktop/src-tauri/Cargo.toml --features mesh-llm check licenses`

The dependency contract test prevents affected parser/build versions or
`cmov 0.5.3` from returning during a later lock refresh. Product behavior,
integration tests, exact-head canonical CI, and independent semantic review
remain separate gates.
