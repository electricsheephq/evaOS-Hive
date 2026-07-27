# Hive upstream and thin patch-stack report

This is the reproducible adoption gate for
[Hive #11](https://github.com/electricsheephq/evaOS-Hive/issues/11). It compares
the exact internal-canary candidate with the current reviewed upstream target
without rebasing or modifying either lineage.

## Exact comparison

| Role | Exact commit |
| --- | --- |
| Pinned upstream base | `dd222a509b156ba52ed3219e895d7bf1cf322c92` |
| Current upstream target | `37420764349bcd8f3dcf34786c30a8f924152922` |
| Hive internal-canary head | `4def81dbea6c3bbd7d375b60836761b828f99574` |
| Hive candidate branch | `fix/36-thin-upstream-parity` |

The machine-readable source of truth is
[`hive-upstream-patch-stack.json`](./hive-upstream-patch-stack.json). It records:

- all 18 upstream commits after the pinned base, each with one terminal
  disposition: `keep`, `redesign`, or `prove-inapplicable`;
- all 37 Electric first-parent commits in the candidate lineage, each with its
  purpose and one terminal replay disposition;
- all six reviewed child commits hidden behind the three merge commits;
- the exact read-only merge-rehearsal tree and seven conflict paths.

Run the static and repository-backed checks from the repository root:

```sh
node scripts/check-hive-upstream-patch-stack.mjs
node scripts/check-hive-upstream-patch-stack.mjs --live
```

`--live` requires the exact upstream target object to have been fetched into the
local repository. It verifies the recorded commit ranges and repeats the
read-only `git merge-tree` rehearsal.

## Adoption decision

Do not blindly rebase the internal-canary branch. The upstream target contains
valuable native product work, including a generic BYOH ACP runtime, but the
rehearsal conflicts exactly where the current Hermes and company-agent adapters
touch native runtime code.

The next adoption branch must:

1. start from a newly pinned upstream commit;
2. inherit all entries marked `keep`;
3. re-express each `redesign` item against the new native seam;
4. replay only the semantic changes marked `keep` or `replay-narrowly`;
5. omit construction-only commits and merge objects marked `drop` or
   `drop-merge-replay-children`;
6. repeat focused tests, exact-head CI, semantic review, artifact proof, and the
   installed canary before making any newer-base claim.

## Upstream disposition summary

| Disposition | Commits | Meaning |
| --- | ---: | --- |
| `keep` | 8 | Inherit unchanged on the next pinned upstream base. |
| `redesign` | 5 | Preserve the observable outcome using the newer native seam. |
| `prove-inapplicable` | 5 | Not part of the current desktop beta; inherit only with the relevant future surface. |

The detailed entry for each commit, including the rationale, is in the JSON
manifest. In particular:

- upstream BYOH ACP (`95fdf978…`) is a chance to shrink the Hermes adapter, not
  permission to conflate dependency readiness with provider authentication;
- upstream agent-definition authority (`8c0e8cb1…`) remains correct for native
  local agents, while VM-hosted company agents stay Hermes-authoritative;
- mobile commits remain outside the current beta until the separate OAuth,
  NIP-AB, pairing, and revocation proof passes;
- TARS, Samantha, and HAL 9000 remain presentation-only starter-persona
  replacements. They do not create company VM profiles or a second prompt
  authority.

## Electric patch-stack summary

The candidate is not a replacement application. Its durable Electric surface is
limited to:

- OAuth, entitlement, durable public identity, and temporary desktop session
  grants;
- host- and company-scoped relay admission and revocation;
- Hive branding and the signed Hive update channel;
- a catalog projection of already-registered public company VM agents;
- native mention/DM discovery for admitted remote agents;
- truthful local-versus-VM runtime presentation.

Generic relay read-back, ACP session continuity, and revocation fixes are
recorded as narrow replay candidates. Construction-only ancestry, transient lint
repair, and merge objects are dropped. The reviewed semantic child commits are
accounted for separately so a later replay never depends on cherry-picking a
mixed merge wholesale.

## Rehearsal result

The exact read-only command was:

```sh
git merge-tree --write-tree --messages \
  4def81dbea6c3bbd7d375b60836761b828f99574 \
  37420764349bcd8f3dcf34786c30a8f924152922
```

It produced tree `8496ebbe06d033ab5515157954cd27f8ac8a6491`
and exited `1` with conflicts in:

- `desktop/playwright.config.ts`
- `desktop/scripts/check-file-sizes.mjs`
- `desktop/src-tauri/src/managed_agents/discovery/tests.rs`
- `desktop/src/features/agents/hooks.ts`
- `desktop/src/features/agents/ui/AgentDefinitionDialog.tsx`
- `desktop/src/features/agents/ui/PersonaModelField.tsx`
- `desktop/src/shared/api/tauri.ts`

This is a bounded native-agent/runtime adapter conflict surface. It is not
evidence that the current candidate is broken, and it is not a reason to change
the installed internal canary before its deferred second-client proof.

## Proof boundary

This report proves commit accounting and a reproducible read-only merge
rehearsal only. It does not prove a merged branch, signed artifact, installed
runtime, customer rollout, or that a future upstream target will have the same
conflict surface.
