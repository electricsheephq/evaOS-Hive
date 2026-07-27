# Hive thin-adapter contribution runbook

Use this runbook for every Electric-only Hive issue, source change, upstream
update, review, or release candidate. Read [`../VISION_HIVE.md`](../VISION_HIVE.md)
first.

## Outcome

Hive remains an easily updated Buzz distribution with a small set of explicit
Electric seams: branding and updates, OAuth and company admission, additive
settings, isolated company relay selection, and registered VM-agent adaptation.

Finishing a task is not enough. The change must preserve native behavior and
remain inside one of those seams.

## 1. Classify the request before changing code

Give the request exactly one primary classification:

| Classification | Default action |
|---|---|
| Native Buzz behavior | Use it unchanged; make no Electric diff |
| Generic Buzz defect | Fix from a fresh upstream base and track as an upstream candidate |
| Electric adapter seam | Add the smallest provider/configuration adapter around the native path |
| Deployment or company operations | Implement in the owning backend, relay deployment, or VM repository; Hive receives only the resulting public contract |
| New non-native product capability | Stop for an explicit owner decision; prefer upstream or the owning external system |

Do not turn a deployment problem into a Hive feature or a native feature into an
Electric service.

The target is a plugin-like boundary, not necessarily a new plugin runtime.
Prefer build configuration and existing providers over creating a framework.

## 2. Resolve authority

Before implementation, write down the source of truth:

- Buzz/Hive owns routine signed collaboration and native product state.
- Electric Sheep owns authentication, company membership, seats, relay
  assignment, the public VM-agent catalog, and company policy.
- The local OS key store owns the human private identity.
- One company relay owns admission and native event enforcement for that
  company.
- Hermes and the VM bridge own agent private identity and runtime configuration.

If two layers would store or mutate the same authoritative state, redesign the
change before coding.

## 3. Prefer an existing seam

Use, in order:

1. existing configuration or build-time product identity;
2. an existing provider or adapter interface;
3. an additive component inside an existing native route;
4. a narrowly contained Electric module;
5. an upstream-generic extension point.

Visible product branding does not require mass-renaming upstream crates,
modules, schemas, protocols, commands, or developer-facing identifiers. Retain
the upstream license and required attribution.

Editing native feature components broadly is a last resort. The issue and PR
must name the reproduced supported-path failure and explain why the earlier
seams cannot solve it.

## 4. Settings rule

Electric settings use one additive Hive/Electric section inside the existing
Settings shell.

Allowed:

- current company and relay identity;
- login/session and device status;
- Hive update-channel and support information;
- registered VM-agent catalog and truthful bridge state;
- the native-equivalent responder policy for registered VM agents.

Not allowed:

- replacing the Settings shell or native pages;
- hiding native features merely because the user is managed;
- copying Hermes model, provider, memory, tool, permission, instruction, or
  secret controls into Hive;
- a second profile/avatar, search, memory, or retrieval subsystem;
- an indefinite loading state without a terminal error or empty state.

## 5. Relay rule

For the guarded business beta, one company receives one relay instance and
hostname.

- Company and relay are selected server-side after OAuth.
- Unknown or ambiguous hosts fail closed.
- Database, cache, object storage, search, secrets, logs, backup, restore, and
  retirement remain company-scoped.
- The client never chooses a tenant or arbitrary relay.
- Native collaboration stays user-signed after admission.
- A control identity is limited to entitlement, membership, administrative
  projection, and clearly labeled system events.

Do not add shared multi-company relay logic to Hive. A future shared deployment
requires its own decision, threat model, operating-cost comparison, and runtime
proof.

## 6. VM-agent rule

Hive consumes already-registered public agent identities.

Allowed:

- merge authorized catalog agents into the native Agents experience;
- display signed profile data or a deterministic fallback;
- show truthful runtime capability and presence evidence;
- derive visible channels from native room membership and user visibility;
- map native allowed-room/allowed-author controls to revisioned bridge policy;
- require VM acknowledgement before reporting policy as applied.

Not allowed:

- enumerate arbitrary VM or Hermes state from the desktop;
- create agents, profiles, services, keys, work directories, memories, tools,
  providers, permissions, or credentials;
- copy or rotate an agent private key;
- claim health, authentication, provider readiness, or channel membership from
  catalog presence alone;
- add Hive-owned channel retrieval or employee-file access.

## 7. Upstream update procedure

1. Record the exact current Hive head, upstream base, and target upstream head
   or release.
2. List every upstream commit between the two bases.
3. Give each commit one candidate-scoped disposition: adopt, backport,
   redesign around, drop, or prove inapplicable.
4. Start from the selected upstream base. Do not blindly rebase a release
   candidate.
5. Replay only the documented Electric seams. Do not cherry-pick mixed managed
   commits wholesale.
6. Compare the resulting patch stack with the previous candidate.
7. Run focused checks first, then canonical CI on the exact pushed head.
8. Obtain one independent semantic review of the changed adapter, identity,
   tenant, agent-authority, and updater surfaces.
9. Keep source/CI, merge, artifact, deployment, runtime, and customer claims
   separate.

## 8. Patch-shape review

The expected Electric diff is configuration plus a few adapter/provider
boundaries. Size alone does not decide correctness, but these are stop signals:

- ordinary channel, DM, profile, search, composer, forum, file, reaction, or
  settings components are being replaced;
- the change introduces a parallel API for a native event flow;
- the same company, identity, membership, policy, profile, or message state is
  authoritative in two systems;
- a managed-mode condition spreads through unrelated native features;
- an Electric change touches several native subsystems to create one outcome;
- a future upstream feature would require a second implementation in Hive.

When a stop signal appears, delete or simplify first. Do not add another broker,
guard, cache, projection, or compatibility layer until a current supported-path
failure proves it necessary.

## 9. Grill the proposal

Answer these questions in the issue or PR when the change is not obviously
configuration-only:

1. What user-visible outcome is unavailable through native Buzz?
2. Is this an Electric admission/branding/agent-adapter concern, or does it
   belong upstream or in another repository?
3. Which layer owns the authoritative state?
4. Can the outcome be delivered by configuration or an existing provider?
5. Does this change duplicate a native screen, event, profile, or workflow?
6. Does it copy private identity, agent runtime, memory, tools, or secrets?
7. Will the next upstream release require us to implement the feature twice?
8. Does it hide a native capability instead of adapting one narrow difference?
9. What exact supported-path failure justifies every native-file edit?
10. What is the smallest runtime proof that the adapter works and fails closed?

An unanswered authority question blocks implementation.

## 10. Required issue and PR content

Every Electric adapter issue names:

- observable outcome and `Done when`;
- exact native surface being preserved;
- adapter seam and owning authority;
- non-goals and prohibited native surfaces;
- focused tests, exact-head CI, semantic review, and installed proof when
  relevant;
- source/CI versus runtime/customer proof boundary;
- stop conditions.

Every PR links its issue and includes:

- exact upstream base and Electric commits;
- files/surfaces changed and why each native edit is unavoidable;
- focused test evidence;
- explicit confirmation that native unmanaged behavior remains unchanged;
- any source-of-truth, artifact, runtime, or customer state not changed.

## 11. Review dispositions

Give each finding one terminal disposition:

- fixed now;
- accepted follow-up with a canonical tracker;
- accepted tradeoff or won't-fix;
- false or not applicable;
- escalated for an owner decision.

Review is bounded risk control, not permission to add hardening or capabilities.
A concern changes the current diff only when it reproduces a supported-path
failure or violates a named invariant.

## 12. Proof and stop conditions

Documentation and tests do not prove an installed product. Use the smallest
end-to-end canary after exact-head CI and review.

Stop immediately on:

- cross-company discovery, messaging, restore, or credential reachability;
- client-selected company or relay authority;
- private-key, provider-secret, VM, connector, or Mac Access exposure;
- durable identity rotation or deletion on logout;
- control-signed routine collaboration;
- duplicated native feature or settings surfaces;
- Hive provisioning or configuring Hermes;
- a new Hive-owned profile, memory, retrieval, or VM-management system;
- an upstream update that cannot be dispositioned without replacing native
  behavior;
- memory/OOM recurrence.

## First-principles checkpoint

- **Desired function:** Electric Sheep customers use a branded Buzz workspace
  with company authentication and their cloud agents.
- **Hard constraints:** tenant isolation, local private-key custody, Hermes
  authority, native feature parity, signed updates, and easy upstream adoption.
- **Magic-wand floor:** product identity configuration, one OAuth/admission
  provider, one additive settings section, one registered-agent provider, and
  one isolated relay deployment contract per company.
- **Current-cost warning:** collaboration brokers, duplicate screens,
  projections, and multi-authority state make the fork several times more
  expensive to maintain than that floor.
- **Recommendation:** delete obsolete parallel paths, simplify adapters, use
  upstream CI/release comparisons, and automate only repeatable comparison and
  deployment evidence.
- **Negative risk:** separate relays cost more to provision and operate; solve
  that in deployment automation rather than by moving tenancy complexity into
  Hive.
