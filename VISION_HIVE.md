# Hive product vision

Hive is native upstream Buzz with a small Electric Sheep product adapter. The
adapter selects who may enter a managed company workspace; it does not replace
Buzz collaboration, identity signing, agent transport, or Hermes.

The exact upstream base for the first v0.5.2-es Internal Canary is block/buzz
v0.5.2 at `3e48f1b2365d326ee1c9582448d86a99b44ecd5d`.

## Native Buzz remains authoritative

Buzz owns channels, direct messages, profiles, search, attachments, reactions,
huddles, notifications, native public-key identities, signing, encryption, and
the Agents transport. Company VM agents use the upstream
`buzz-acp -> hermes-acp` path. Hermes owns model, memory, tools, provider,
permissions, and runtime configuration.

Hive does not broker ordinary collaboration events or create a second profile,
avatar, message, room, or agent-runtime authority.

## Electric Sheep adapter

Electric Sheep adds only the seams upstream cannot provide:

- Hive branding, bundle identity, signed updater, and support destination.
- Electric OAuth, active company membership and seat checks, and the
  server-selected company community and relay.
- A revocable desktop session entitlement in addition to the native Hive key.
- Automatic recovery of the same durable native identity on an authorized Mac.
- The smallest proven company-agent authorization and presentation projection,
  keyed by an existing public agent identity.

Invites remain on the Electric Sheep website. Each company has one managed
relay, VM, and Hermes gateway, with separate employee profiles. Support has no
standing relay identity.

## Durable managed identity

Managed relay access requires both:

1. a current Electric OAuth-derived company/session entitlement; and
2. possession of the membership's canonical native Hive private key.

The private key still signs native Buzz events and decrypts native DMs. Logout
revokes the Electric desktop session only; it does not rotate or remove the
durable recipient identity.

For a Mac that does not yet hold the canonical key, Electric stores one
recoverable encrypted envelope in Supabase. A random data-encryption key wraps
the raw 32-byte Nostr key with AES-256-GCM using membership, community, and
identity scope as authenticated context. The server-side Supabase Edge-secret
keyring wraps that data-encryption key. After fresh OAuth, the server releases
only ciphertext plus a one-time HPKE P-256 sealed data key to the requesting
native process. Hive verifies the recovered public key and access revision,
stores the key in macOS Keychain, and completes the normal signed key challenge.

Private-key plaintext never enters renderer state, browser storage, relay
state, logs, analytics, GitHub, or evidence. No AWS SDK, KMS ARN, external
custody provider, relay-side signing, or raw NSEC recovery screen is part of
managed Hive. Unmanaged Buzz retains its native identity and pairing behavior.

## Release proof

Source tests and CI prove only source behavior. Merge, backend deployment,
signed artifact production, installed Andrew/Benjamin recovery, cumulative
channels/DMs/agents/huddles/reconnect/memory proof, internal distribution, and
customer rollout are separate claims. David and customer distribution remain
blocked until the cumulative Internal Canary is accepted.
