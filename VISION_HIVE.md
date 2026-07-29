# Hive — Electric Sheep's thin Buzz distribution

Hive gives an Electric Sheep company a private, branded workspace where people
use native Buzz collaboration and work with the agents registered for their
company. It should feel like Buzz with Electric Sheep sign-in and cloud agents,
not like a second product built on top of Buzz.

Upstream [`VISION.md`](VISION.md) remains the product vision. This document
defines only the Electric Sheep distribution boundary.

## The customer experience

1. A person signs in with their Electric Sheep account.
2. Electric Sheep selects the person's company and that company's relay.
3. Hive opens the native workspace with the person's durable Hive identity.
4. Channels, DMs, profiles, search, files, forums, reactions, settings, and
   other collaboration features behave as upstream Buzz designed them.
5. Agents already registered for that company appear through the native Agents,
   channel, mention, and DM surfaces.
6. Electric-specific configuration appears as a small additive section inside
   the existing Settings structure.

Signing out ends the device session. It does not delete the person's durable
recipient identity or prevent offline messages from accumulating.

## Product boundary

Hive owns only these seams:

- **Product identity:** visible Hive name, icon, support destination, bundle
  identity, and signed Hive update channel while retaining required upstream
  license and attribution.
- **Bundled persona presentation:** unmanaged Buzz keeps the native bundled
  starter personas. Managed Hive presents the canonical company VM identities
  for TARS, Samantha, and HAL 9000 and retires only the duplicate built-in local
  starter presentation and start path. Retained local records are not deleted,
  and explicit user-created local agents keep native behavior when their
  runtime is supported.
- **Company admission:** Electric OAuth, membership and seat entitlement,
  server-selected company, and server-selected relay.
- **Local identity custody:** the person's private key remains in the operating
  system key store; Electric systems store public identity and session metadata.
- **Company configuration:** a small additive Settings section for options that
  differ from upstream.
- **VM-agent adaptation:** map already-registered public agent identities and
  supported responder policy into native Hive surfaces.
- **Deployment:** one isolated relay instance per company for the guarded
  business beta.

Everything else remains native Buzz unless an exact supported-path failure
proves that a narrow adapter is required.

"Thin adapter" describes the architectural boundary. Hive does not need to
invent a dynamic plugin framework when build configuration, existing providers,
and small additive components already provide that boundary.

## Authority boundaries

| Authority | Owns |
|---|---|
| Buzz/Hive | Native collaboration, events, channels, DMs, profiles, search, files, settings, and user experience |
| Electric Sheep backend | Human authentication, company membership, seats, relay assignment, public VM-agent catalog, and company policy |
| Electric Sheep release process | Hive update signer, signing-key custody, rotation and revocation, signed appcast publication, and release evidence |
| Local OS key store | Human private Hive identity |
| Company relay | Admission plus native event and room enforcement for exactly one company |
| Hermes and the VM bridge | Agent private identity, runtime, model, provider, memory, tools, instructions, permissions, secrets, and process isolation |

No layer may silently take authority from another. In particular, Hive must not
copy Hermes state or turn Electric services into a broker for routine messages.

The signed-update gate is the exact-tag release workflow in
[`release.yml`](.github/workflows/release.yml). Release evidence must bind the
source head and tag to the signed and notarized application, detached updater
signature, appcast entry, and a clean install/update verification. If artifact
or update-signature verification fails, the release is not published or
installed; the client keeps the last verified installed version. Signing-key
rotation or revocation is owned by the Electric Sheep release process and
requires a separately recorded release decision and verification.

## One company, one relay

During the internal canary and guarded business beta, each company receives one
customer VM containing that company's relay and Hermes gateway. The company's
people use separate Hermes profiles on that VM. Different companies do not
share a VM or relay.

Each relay has a separate hostname. Database, cache, object storage, search,
secrets, backups, restore targets, and operational lifecycle are scoped to that
company.

This costs more operationally than shared infrastructure, but it makes
cross-company isolation structural and keeps multi-tenant complexity out of the
Hive client. A shared multi-company relay requires a separate future owner
decision, cost case, threat model, and runtime isolation proof.

## Native first

When upstream adds or changes a feature, the default answer is to inherit it.
Hive should not:

- mass-rename internal Buzz crates, modules, protocols, schemas, or developer
  terminology merely to change the visible product name;
- duplicate an upstream screen or workflow;
- hide or disable native features without an explicit product disposition;
- replace native channels or DMs with managed APIs;
- sign routine collaboration with a control identity;
- create a second profile, avatar, search, memory, or retrieval system;
- provision or enumerate Hermes profiles, services, keys, work directories,
  memories, tools, providers, or credentials;
- expose customer VM, provider, Keychain, Mac Access, or connector secrets;
- accept a client-selected company, tenant, relay, agent, or policy authority.

If a proposed change needs any of those patterns, stop. First prove why the
native path and the documented adapter seams cannot deliver the outcome.

## Agent model

Company VM agents are real Hive identities registered by their owning runtime.
Hive may display their public identity, truthful capability and presence
metadata, visible native room membership, and an adapter for the native
responder-policy controls.

Hive does not create or configure company VM agents. A catalog refresh
discovers only identities already registered by an authorized server-side
process. ATRIS and other VM-agent names are catalog/profile data.

TARS, Samantha, and HAL 9000 are canonical company VM profiles in managed Hive.
When those registered identities are available, Hive presents them through the
same catalog adapter as other VM agents and suppresses the duplicate built-in
local welcome personas from managed pickers, search, and manual-start controls.
The retained local records remain untouched. Unmanaged Buzz keeps its native
starter team, and explicit custom local agents keep native creation, editing,
runtime, sandbox, permission, and storage behavior when supported.

## Settings model

Electric configuration belongs in one additive Hive/Electric section within the
native Settings navigation. It may show company, relay, session, update,
support, and registered-agent information that genuinely differs from upstream.

It must not fork the Settings shell, replace native pages, or use a managed mode
as a reason to hide unrelated upstream capabilities. Unsupported features must
be labeled truthfully and narrowly.

For the current release, company invitations and seat changes remain in the
Electric Sheep website's People/Access experience. A future Hive integration
may adapt the native People/Access surface to that backend, but must not create
a second invitation or membership authority.

## Support model

Electric Sheep has no permanent support identity inside a customer relay and no
default access to customer messages or VM state. Current support is
customer-controlled screen sharing or remote control of the customer's visible
Hive session.

Any future cross-company super-admin or support view requires a separate owner
decision, explicit customer policy, audit, revocation, and security review. It
is not part of the current adapter.

## Staying close to upstream

Every Hive candidate records an exact upstream base and a small, explained
Electric patch stack. New upstream commits are dispositioned before adoption;
the fork is never blindly rebased into a release.

Generic fixes are developed from a fresh upstream base. Electric branding,
OAuth, endpoints, company policy, and VM details never enter an upstream
candidate.

The contribution process and stop rules are defined in
[`docs/hive-thin-adapter-runbook.md`](docs/hive-thin-adapter-runbook.md).

## Success

Hive succeeds when Electric Sheep customers receive a reliable Slack-like
workspace with their company agents, while new Buzz releases remain inexpensive
to evaluate and adopt.

The proof is not the size of this document or the number of guardrails. The
proof is that native features keep working, the Electric patch stack stays
small and isolated, company boundaries fail closed, and installed customer
flows work on an exact reviewed release.
