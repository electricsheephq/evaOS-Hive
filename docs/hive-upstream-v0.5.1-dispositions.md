# Hive #69 — Buzz v0.5.1 upstream disposition gate

This is the no-blind-rebase planning gate for adopting the pinned Buzz
`v0.5.1` source history. The JSON manifest is the machine-readable source of
truth; this report makes the 104 decisions and the thin Electric replay
groups reviewable.

## Exact source boundary

| Role | Exact identity |
| --- | --- |
| Pinned upstream base | `dd222a509b156ba52ed3219e895d7bf1cf322c92` |
| Buzz v0.5.0 | tag `v0.5.0` at `4a977c588a540be38bd8ddb268cd24437bac8165` |
| Buzz v0.5.1 candidate | tag `v0.5.1` at `a13085e9ac9a7c8dbd9426a6b88fc75abf62220e` |

The ordered range is exactly 104 unique commits: 67 from the pinned base
through v0.5.0, then 37 from v0.5.0 through v0.5.1.

## Disposition totals

| Disposition | Count | Meaning |
| --- | ---: | --- |
| `keep/adopt` | 91 | Inherit the upstream source while preserving the recorded managed-boundary risk. |
| `managed-surface omit while retaining upstream` | 7 | Keep native source but omit the named managed Hive presentation or authority. |
| `redesign/narrow replay` | 3 | Preserve the needed outcome through the current native seam; do not cherry-pick blindly. |
| `prove-inapplicable` | 3 | Account for history that has no target-tree effect or is neutralized inside the range. |

## All 104 upstream commits in exact Git order

Rationale and candidate risk are recorded for every row in
[`hive-upstream-v0.5.1-dispositions.json`](./hive-upstream-v0.5.1-dispositions.json).

| # | Commit | Exact subject | Segment | Disposition |
| ---: | --- | --- | --- | --- |
| 1 | `74b63e1846212af6e6751a62cfc631f74b1dfe07` | Refactor managed-agent runtime into cohesive modules (#2974) | `base-to-v0.5.0` | keep/adopt |
| 2 | `c9a73726be6e6edd4e5d0c9c44abdf07d550ecd0` | fix(mobile): validate invite relay destinations (#2986) | `base-to-v0.5.0` | managed-surface omit while retaining upstream |
| 3 | `166c6655e8bca87d83ad60c087fb70a32a026baf` | fix(desktop): surface install failures hidden by curl-pipe exit codes (#2892) | `base-to-v0.5.0` | keep/adopt |
| 4 | `8e67cf399d0291bcdbc69cd0402983ca030f05bb` | chore(desktop): delete dead persona catalog UI cluster (#2886) | `base-to-v0.5.0` | keep/adopt |
| 5 | `8c0e8cb1656b04ad269bce3c2deeda2a943ae78a` | fix(desktop): make agent definition authoritative for model/provider/prompt (#1968) | `base-to-v0.5.0` | keep/adopt |
| 6 | `dc1646fcb952c8fb9ab62b838a5ccfb7bfdbbaae` | docs: document required DCO sign-off and add commit-msg sign-off hook (#2993) | `base-to-v0.5.0` | keep/adopt |
| 7 | `a31fc4d2f35d51cdf45ff8c61fc3a07f49c665e8` | fix(desktop): remove bundled libsystemd from AppImage (#2353) | `base-to-v0.5.0` | keep/adopt |
| 8 | `e6c90bb7c430d1b2af16508b634f9a5283b7fa3b` | Polish community rail and mobile pairing (#2972) | `base-to-v0.5.0` | managed-surface omit while retaining upstream |
| 9 | `1a56b7cc9e441819328ae6814d15f83ed22388f8` | feat(mobile): add worktree-aware debug identities (#2858) | `base-to-v0.5.0` | keep/adopt |
| 10 | `5d8ede446f8fdc48146fe56d389cab6bf3500f92` | feat(agents): lower default agent parallelism from 24 to 10 (#3038) | `base-to-v0.5.0` | keep/adopt |
| 11 | `871a3b37723404a34c4cdf63037bbc4916b25245` | Refine mobile community switching and discovery (#2967) | `base-to-v0.5.0` | managed-surface omit while retaining upstream |
| 12 | `aee63144843854ee32ed9d36a2e7511c82ddc6b0` | fix(desktop): strip legacy baked team instructions from stored prompts (#3035) | `base-to-v0.5.0` | keep/adopt |
| 13 | `16d4ec335e210295a9d9f77f36c1e85a18b6814a` | feat(desktop): use collective mesh routing for Auto (#2825) | `base-to-v0.5.0` | managed-surface omit while retaining upstream |
| 14 | `95fdf978800982389b120c66ff5e766d785419c7` | feat(acp): bring your own harness (BYOH) — generic ACP runtime seam + settings gallery (#2773) | `base-to-v0.5.0` | keep/adopt |
| 15 | `63c62fcf3eb5321262e8b7c4e299d110de330884` | chore(deps): update dependency @tanstack/react-virtual to v3.14.8 (#3057) | `base-to-v0.5.0` | keep/adopt |
| 16 | `070fb6a16153b47d88953e1a2db5c2f241b838b5` | chore(deps): update radix-ui-primitives monorepo (#3063) | `base-to-v0.5.0` | keep/adopt |
| 17 | `afb272bb7b8d7d45d7de676fa97dcd5a8eefacc7` | fix(desktop): render rich project work item content (#3100) | `base-to-v0.5.0` | keep/adopt |
| 18 | `37420764349bcd8f3dcf34786c30a8f924152922` | fix(mobile): mitigate message-post delay with optimistic rendering (#3037) | `base-to-v0.5.0` | keep/adopt |
| 19 | `7fc0cc82db4d9dced9c258bbe8b530164a832a77` | Restore Goose and Buzz Agent to onboarding harness selection (#2731) | `base-to-v0.5.0` | keep/adopt |
| 20 | `87b3fcd3c0131683569dd4268b099d18b25dcd5e` | fix(desktop): clarify identity key button when key exists (#2357) | `base-to-v0.5.0` | keep/adopt |
| 21 | `c5c4f390b6713256e2efb8394c59823ebad73db6` | feat(desktop): handle project work from Inbox (#3117) | `base-to-v0.5.0` | keep/adopt |
| 22 | `00ecf2cac7544d986b4eb111ad0a8b1d7560791f` | fix(security): authorize kind:9000 role changes in both directions (#3017) | `base-to-v0.5.0` | keep/adopt |
| 23 | `654f384906b5c720a60a199d85031a6f1cb6efc9` | fix(desktop): read the newest pair-scoped harness log (#3134) | `base-to-v0.5.0` | keep/adopt |
| 24 | `4ef5f10ce4937b4e7d7059de22cabf78b616437b` | docs(contributing): set PR expectations and require UI screenshots (#3140) | `base-to-v0.5.0` | keep/adopt |
| 25 | `31e2de1966672e73e026af3c54f3a1a9a2f5e103` | fix(deps): bump nostr to 0.44.6 for RUSTSEC-2026-0216 (NIP-44 remote DoS) (#3135) | `base-to-v0.5.0` | keep/adopt |
| 26 | `fe84274d215b098a97b0b46cf565d736dbce14b7` | chore(deps): update react monorepo to v19.2.8 (#3064) | `base-to-v0.5.0` | keep/adopt |
| 27 | `e2e007910114ddf7c5a4e93bb03f6afe13552e92` | fix(security): enforce durable community ban on NIP-43 relay-admin kinds 9030-9033 (#3128) | `base-to-v0.5.0` | keep/adopt |
| 28 | `18eef633d88ac465c61d98f12655fbf51dc3ca44` | feat(git): use agent display name as git author name (#3040) | `base-to-v0.5.0` | keep/adopt |
| 29 | `68f39f36972f319b91e0d93e945d7f17b3269a82` | feat(mobile): bring message actions to desktop parity (#3070) | `base-to-v0.5.0` | keep/adopt |
| 30 | `f2fe3b63c21be55907175715c076cd3a9195b74d` | feat(acp): title agent sessions from the agent and channel name (#3028) | `base-to-v0.5.0` | keep/adopt |
| 31 | `be275cfc6c7b80fe43e9d66c6d14b6d2bbe58a10` | fix(desktop): keep identity key help dialog readable in dark mode (#2854) | `base-to-v0.5.0` | keep/adopt |
| 32 | `313f793c8753d413c22ff8edfe420d5ee78708bc` | feat(desktop): add search to agent emoji picker (#2630) | `base-to-v0.5.0` | keep/adopt |
| 33 | `9d36778c37e42318135e92ccb0e4491afac34a17` | feat(mobile): refactor Activity behavior and ui (#2889) | `base-to-v0.5.0` | keep/adopt |
| 34 | `545bb46b824a3fbf4401062f03b72531d832ebb9` | fix(desktop): make lint and unit-test gates work on Windows (#2943) | `base-to-v0.5.0` | keep/adopt |
| 35 | `8bb43d51912894553f2670b2d285a96cf09cd472` | fix(desktop): make the test loader work on Windows (#2758) | `base-to-v0.5.0` | keep/adopt |
| 36 | `c3084b36d975259f2dfeee8edc9131b40a8bce83` | fix(cli,relay): resolve agents by verified owner (#2615) | `base-to-v0.5.0` | prove-inapplicable |
| 37 | `b92a1f4bf400e7da5ab7a010cdd81a69497d8191` | chore(desktop): add AgentCreationPreview file-size override to unblock main CI (#3154) | `base-to-v0.5.0` | keep/adopt |
| 38 | `e28707f6b24a67284276bc29d4dc36b4f53ad53c` | fix(mobile): retry channel-sections startup sync when relay rate-limits cold start (#3004) | `base-to-v0.5.0` | keep/adopt |
| 39 | `32ead931011bd95d3423d4db17b087fe110808a4` | fix(mobile): tapping threaded message in Inbox navigates to top level of channel  (#2103) | `base-to-v0.5.0` | keep/adopt |
| 40 | `8995316844f7ad50552fbae67fbd35119262796f` | fix(desktop): use forward slashes for git credential.helper on Windows (#3023) | `base-to-v0.5.0` | keep/adopt |
| 41 | `3faea98891ce43b772dc94366a45f1c3b99d102c` | fix(mobile): match markContextRead signature in activity test fake (#3158) | `base-to-v0.5.0` | prove-inapplicable |
| 42 | `d98da7389e60cfbd79b219aa411449fe2e53a18a` | feat(desktop): redesign agent runtime settings (#3093) | `base-to-v0.5.0` | keep/adopt |
| 43 | `a041e2d21e292a271fdfc26f0cdcdd0456f815c5` | Revert "fix(cli,relay): resolve agents by verified owner" (#3168) | `base-to-v0.5.0` | prove-inapplicable |
| 44 | `9b0f744804697b802f7afb88947194702765c78d` | resolve findings (#3150) | `base-to-v0.5.0` | keep/adopt |
| 45 | `137185e056c469ff613efc16f88044bc036a9dc6` | chore(deps): update plugin org.jetbrains.kotlin.android to v2.2.21 (#3058) | `base-to-v0.5.0` | keep/adopt |
| 46 | `4d8b676bb283a1917cec5850c3b7327fe122b0c1` | fix(desktop): keep collapsed table separators out of spoilers (#3169) | `base-to-v0.5.0` | keep/adopt |
| 47 | `01c23810fa7fcf5881101d45543dd2cfdc193608` | Replace mobile reconnect banners with skeleton shimmer (#3143) | `base-to-v0.5.0` | keep/adopt |
| 48 | `174c38e4bd1ed8498641546bc4fcb6d5a4c9cede` | fix(desktop): recover full local storage on startup (#3182) | `base-to-v0.5.0` | keep/adopt |
| 49 | `f069a8550373328babe4239ed614fcdf884721e2` | feat(admin): show reported message content in report detail (#3149) | `base-to-v0.5.0` | keep/adopt |
| 50 | `75588eaff2354d620e554c055b80ec83735ddb0a` | Refine pending message status (#3153) | `base-to-v0.5.0` | keep/adopt |
| 51 | `99da5b7ebb19e26453e075bfb949672122b31be3` | Fix composer selection formatting and drop overlay (#3172) | `base-to-v0.5.0` | keep/adopt |
| 52 | `2bd4c24b71335e7ce272ec6de6491f7f37f4b20d` | Inbox refactor (#2045) | `base-to-v0.5.0` | keep/adopt |
| 53 | `de13960505fd798070e177cb33b1663100ac06bb` | fix(desktop): keep project Inbox previews compact (#3193) | `base-to-v0.5.0` | keep/adopt |
| 54 | `7ca0bbd946fd82a7008132f94d069a97bb53f94b` | fix(desktop): republish agent identity records when a persona rename propagates (#2607) | `base-to-v0.5.0` | keep/adopt |
| 55 | `0019f80765e96f056e81b57789b8b5fb80936f72` | fix(desktop): fetch join policies through native networking (#2862) | `base-to-v0.5.0` | managed-surface omit while retaining upstream |
| 56 | `cb2a265b5399426e808461c1a16713754c593258` | feat(search): parse from:/in:/after:/before: and pass them in the filter (#2871) | `base-to-v0.5.0` | keep/adopt |
| 57 | `9810d8545937329f229ff40d8a19edc9e3e325c1` | fix(desktop): preserve thread anchor through layout reflow (#3212) | `base-to-v0.5.0` | keep/adopt |
| 58 | `98a7b1334823ee0be3e3fa5cab7a2e349e438dab` | fix(node): bump Buzz-supplied Node runtimes past OpenClaw's >=24.15.0 floor (#3218) | `base-to-v0.5.0` | keep/adopt |
| 59 | `d500c2d5cf5d9aabe0ca4ebebfcafdbe5f5b7fd3` | feat(invites): add use-limited invite links (#3141) | `base-to-v0.5.0` | managed-surface omit while retaining upstream |
| 60 | `d8f9d87c17131b952ea5b6c3767978c4637545fc` | Polish composer activity layout and transitions (#3151) | `base-to-v0.5.0` | keep/adopt |
| 61 | `94675e0d2518a0324d1efdfa5c0aeb8a367b3016` | refactor(desktop): extract install command execution into install_exec (#3251) | `base-to-v0.5.0` | keep/adopt |
| 62 | `be13b4bb9ce228b21fa3682ce75d75cba5950561` | fix(desktop): probe legacy Goose install dir on Windows (#3248) | `base-to-v0.5.0` | keep/adopt |
| 63 | `925a9a7bf230e67a18abc8fc7996fa39d620846b` | fix(buzz-acp): accept id-keyed config options when resolving model switch (#2795) | `base-to-v0.5.0` | keep/adopt |
| 64 | `e94b9aeda0b2272d36e3744e78680be69295b8b5` | feat(tracing): add datastore tracing plumbing (#2760) | `base-to-v0.5.0` | keep/adopt |
| 65 | `3a4bf513df0e0c258587bfcbed9463d63723b56b` | Publish symbol-bearing debug relay images (#3250) | `base-to-v0.5.0` | keep/adopt |
| 66 | `2ce2d71cc38a9657eaf344c10e07f155b8a18615` | feat(relay): make Postgres pool size configurable, default 50 (#3191) | `base-to-v0.5.0` | keep/adopt |
| 67 | `4a977c588a540be38bd8ddb268cd24437bac8165` | chore(release): release Buzz Desktop version 0.5.0 (#3213) | `base-to-v0.5.0` | redesign/narrow replay |
| 68 | `7dfea2634f7e87f6a42f5fc1f22d9f77c648abfc` | Add mobile message image galleries (#3312) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 69 | `6da45ac5cf90fa0768a98256e2200708d219ddfc` | Polish mobile message and search layouts (#3121) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 70 | `a3b097745a3fc22872d05bbd558d231ead4e661d` | Refine mobile attachment picking (#3313) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 71 | `af4d8615165b9bdbe1190d4ba71ff32b1df75a8a` | feat(chart): add relay pod extension points (#3322) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 72 | `5457c947a74f5ba4b979f9c6411aa7626a858387` | fix(composer): scope multiline block formatting (#3246) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 73 | `60158fce3e670f11bb35d42627857ccaea50ff06` | feat(cli): add users set-status command for NIP-38 profile status (#3253) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 74 | `4e3998f36e36d68b9a93dcbd85f0864450bb8f5f` | fix(desktop): gate codex-acp on a minimum supported version (#3254) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 75 | `00ede2e7aa7eb95571b7db3ebbd163adbf6cf74e` | fix(desktop): restore the inbox icon in the sidebar (#3341) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 76 | `a77212875aea299350d01d94b0f6d9c22a8fce5f` | Unify mobile loading spinners (#3314) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 77 | `35305bfc8fd456ca9a17caa1ddbfaabd87d46981` | docs: restructure DCO guidance into scannable subsection (#3337) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 78 | `3afa129ee785cc74d921d0ba969254a8255c4cc0` | fix(desktop): keep drafts out of the Inbox All view (#3217) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 79 | `a35771fc441cdc3c6f517f419037206783b502d2` | feat(desktop): refine agent catalog sharing (#2439) | `v0.5.0-to-v0.5.1` | redesign/narrow replay |
| 80 | `1e307e178a7b7fc157cde1ef0721ba1a69cbc274` | chore(compose): remove stale typesense env vars (#3332) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 81 | `b0503d80c298b1ece3b0a43b41d316829a3379e7` | feat(desktop): add custom harness inline from agent dialogs (#3252) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 82 | `1d4f97b959a0d91f7bac0e1f97189e5c10347712` | fix(acp): disable goose cron scheduler in managed agent children (#3144) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 83 | `1d3b810ad70d6325718ed91e723f32c4a376d5e1` | fix(desktop): paint community rail full height (#3382) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 84 | `4fcd55a9991a02dd41c0637b14e5cb0cd88b992e` | docs(contributing): document the Linux system libraries just ci requires (#3396) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 85 | `913d564ce0f35924291bf3eeab6508517a6d8d1f` | fix(desktop): stabilize flaky DM expansion E2E ordering assertions (#2004) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 86 | `3ece4461df8a7b9663a8e68327483b8377d4086d` | feat(desktop): apply WebKit rendering workarounds at startup on Linux (#3271) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 87 | `f25e6dd6aa4a1c5eff0facc260b8e25d05a2b02a` | feat(acp): steer claude-code and codex agents via _session/steering (#3007) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 88 | `12d63c67be276d9d11a436374b61469a00bb3808` | release(chart): publish 0.1.7 (#3393) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 89 | `826bed4821f035841193b7660655736171e66211` | chore(ci): bump desktop smoke E2E timeout to 30 minutes (#3409) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 90 | `9227bdf58ad6664ae3c1078888f2181ec19c4da4` | fix(ci): ratchet file sizes against the base tree (#3352) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 91 | `55a3ed7b9217cee5b23e0a5441947dc929b2a38c` | fix(desktop): clear stale thread new-message pill (#3411) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 92 | `90e058ebf68137e048a409aec6616519379ff726` | feat: add explicit entry for claude-opus-5 in model config (#2831) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 93 | `22be8bb35177e27efc2dca2534df9a8dd871eae0` | fix(relay): avoid subscription lock inversion (#3413) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 94 | `6300a6b1d03e32c473c7b6568df663c8927565cf` | fix(acp): per-runtime env defaults at spawn — isolate Hermes from configured MCP startup (#3420) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 95 | `485d03a358b6d695aaf97879f3fbaf2f308d0755` | Fix mobile attachment and gallery polish (#3370) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 96 | `c405ad1d4b1da061c11b3d26761252d41dcc62d3` | feat(agent): fix Anthropic prompt caching with Databricks (+ MCP proxy/TLS passthrough) (#3463) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 97 | `ce01e930edff97bdc12a205fb8f938fcacdba8c1` | Polish mobile typing indicator (#3528) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 98 | `24d90d1280a9325c6cbcf8eea30ac54db5afd2cb` | Refine community invite limits (#3529) | `v0.5.0-to-v0.5.1` | managed-surface omit while retaining upstream |
| 99 | `6438dedf83a9dbe1853e484326911bf6c7f1618c` | feat(agent): route Claude/GPT model families to their native gateway wire (#3538) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 100 | `4555899ab2bfc7aa47b22dd872253f3704091782` | Polish mobile navigation and menus (#3486) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 101 | `f7a3988ba13b590d9a55a7e8413fc3fb5ffbef18` | fix(desktop): preserve shared agent fidelity (#3553) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 102 | `294c8c821de51442a8c384c0bdb66b1a10224ca0` | perf(desktop): move observer-feed archive and decrypt commands off main thread (#3415) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 103 | `51bb97d2be658854bb8a39983af568e90591d375` | Run Tauri clippy in pre-push (#3555) | `v0.5.0-to-v0.5.1` | keep/adopt |
| 104 | `a13085e9ac9a7c8dbd9426a6b88fc75abf62220e` | chore(release): release Buzz Desktop version 0.5.1 (#3566) | `v0.5.0-to-v0.5.1` | redesign/narrow replay |

## Thin Electric replay groups

These groups are planning inputs from the Electric replay map. They are not
instructions to cherry-pick the listed commits and do not prove that any
change applies to, compiles on, or runs on v0.5.1.

| Order | Group | Source commits to mine | Disposition |
| ---: | --- | --- | --- |
| 1 | Hive package/product contract | `94d210b2c1bac78745ef490bfb362bce8234c425`, `bd70ae07ebb40d1f57d64fee51424f2e15a995ef`, `4def81dbea6c3bbd7d375b60836761b828f99574` | Replay narrowly against v0.5.1 package and updater seams. |
| 2 | Managed OAuth, entitlement, and verified admission | `ecca4b4e95014b100a04b86cec5b0719eab09ac4`, `e0cce047e9f7c1b91175498275b12d574a2cab27`, `36a22e8d9ad5c51370d9e29a62dca836276078b6`, `12c3ef3a8d476b57aec9f2e05599811322fe1341`, `64191bd4e6f1f466f472201c21ef19e5f6228037`, `33c84632424f11d85686dffe1e0945737045e3d3` | Redesign as a thin proof-bound adapter; leave managed identity recovery to Hive #78. |
| 3 | Native community bootstrap from verified entitlement | `68bbbfdfdefa8beadbc0d4f00349ef959685ab47`, `50e04c7a43ee9d88f182dcaaff0b5d3a1d63b2c7`, `913f179ba3e6a0b37ef050566a1130500b4c2890` | Replay narrowly through Buzz native community state; no Hive collaboration broker. |
| 4 | Company member and VM catalog provider | `bb20a637d26fe90a2938e0297639c152e4caeb54`, `cad272133701f1a887b504daf6dcbfcb13ffed0e`, `22395ab086a5c971cfa7599215383c1d0b19ad59`, `11e36356765d7e9e9d7d2eb67b7654d9a593daf2`, `e9fd62a066f9a73a70a2b247e67fe135cee79a5b`, `019f56453a1453727f3ea5bd0092d5cf038530d9`, `cd4a0c3409e628a1caff12de511c91010d35787d`, `519d4dc6e15a9824f5ea288a90249251b0237247`, `dcd34b8510b8e4015898a01687a847a961c59b9a`, `ce2850ca75a4d528baa4280953331b13188664a2` | Redesign as one tenant-scoped provider of native Buzz records. |
| 5 | Independent native fixes | `af35265f2c745de634361976076bfcc421a6ad6e`, `e4565f423fa72f8cd68ae44bcd4b43db98fde516` | Replay narrowly only after a v0.5.1 source check and keep them separate from adapter code. |
| 6 | Packaging and adapter proof | `cb2d74e8922a161b7cca966fccff27d78a19a069` | Keep only if the current Hive deployment still consumes the exact artifact; re-author tests for the new seams. |

## Reproducible check

From the repository root:

```sh
node scripts/check-hive-upstream-v0.5.1-dispositions.mjs
```

The check resolves the pinned base and both local tags, verifies ancestry,
compares every full hash and exact subject with `git log --reverse`, validates
the allowed terminal dispositions and required rationale/risk fields, and
enforces the 67 + 37 segment split.

## Proof boundary

This artifact is planning and source-history proof only. It proves exact
commit accounting and the recorded disposition decisions. It does **not**
prove thin-adapter adoption, tests, CI, merge, signed or notarized artifacts,
appcast or other publication, deployment, runtime behavior, customer
installation, release, or adoption.
