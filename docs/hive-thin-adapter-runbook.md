# Hive thin-adapter runbook

> **August 2026 scope: INTERNAL ONLY.** ElectricSheep-internal dogfood of the
> v0.5.2-es Internal Canary only — zero customer exposure, no guarded beta, no
> general rollout, mobile and Windows out of scope. The beta and 1.0 milestones
> are deferred (not dropped) to the September readiness read. Scope statements
> for the Electric adapter live here and in `VISION_HIVE.md`; the other
> `VISION_*.md` files are upstream Buzz documents and are deliberately left
> unmodified to keep the adapter thin.

This runbook covers the Electric-only adapter on the exact upstream Buzz
v0.5.2 base. It is not authority to replace native Buzz features or Hermes.

## Build feature boundary

The Electric adapter is a compile-time feature. Release and canary builds must
pass `evaos-teams-managed`; setting `EVAOS_REQUIRE_MANAGED=1` makes a missing
feature fail the build instead of producing a plain Buzz artifact (issue #92).

## Boundaries

- Buzz owns native collaboration, identities, signing, encryption, huddles, and
  agent transport.
- Hermes owns company-agent model, memory, tools, provider, permissions, and
  runtime.
- Electric OAuth selects one active company membership, community, relay, role,
  access revision, and canonical public identity.
- Managed relay admission requires both a valid Electric desktop session and a
  valid signature from that canonical native identity.
- The renderer may receive only the safe entitlement projection. Opaque
  sessions, recovery challenges, device keys, data keys, and private-key
  material stay in the native process or approved server secret store.

## Managed sign-in paths

### Matching Keychain identity

1. Complete Electric OAuth and claim the one-time desktop device code.
2. Read the server-selected membership and canonical public key.
3. Sign the normal native key-binding challenge with the matching Keychain key.
4. Validate the returned community, relay, public key, role, access revision,
   and expiry before enabling managed relay access.
5. If no custody envelope exists yet, create it through the bounded enrollment
   flow below.

### Missing or mismatched local identity

1. Complete a fresh Electric OAuth desktop session.
2. Generate an ephemeral native P-256 HPKE device keypair.
3. Request the one-time membership-bound recovery challenge.
4. Open the HPKE-sealed challenge and consume it once.
5. Obtain the encrypted identity payload and HPKE-sealed data key.
6. Issue the normal server-selected native key challenge.
7. Validate the recovery identity, membership, community, public key, key
   epoch, access revision, expiry, cipher suite, authenticated context, and
   payload digest.
8. Decrypt in the native process and verify that the derived public key exactly
   matches the canonical membership identity.
9. Sign and verify the normal key challenge in memory.
10. Only after successful server verification, write and read back the key from
    macOS Keychain. Managed recovery never creates a plaintext fallback file.
11. Enable the validated entitlement. If any step fails, revoke or abandon the
    pending desktop session; never enroll a new key as fallback.

Managed UI shows Electric sign-in or retry only. It must not show Mobile,
NIP-AB, `nostrpair://`, or raw NSEC recovery. Unmanaged Buzz remains unchanged.

## Envelope enrollment

After the current native identity and entitlement are proven:

1. The native process requests a short-lived enrollment challenge and ephemeral
   HPKE-sealed data key.
2. It encrypts the raw 32-byte private key with AES-256-GCM. The canonical JSON
   custody context includes the application, identity, membership, community,
   and envelope schema and is used as authenticated data.
3. It signs the exact enrollment challenge as native Nostr kind `27235`.
4. The server verifies the signature, one-time challenge, session, membership,
   access revision, payload digest, and custody-context digest before storing
   the ciphertext and wrapped data key.

The Supabase Edge-secret keyring is a server-only envelope wrapper. No AWS or
external KMS runtime/configuration is required. Backend source merge does not
prove its migration, Edge secret, or function has been deployed.

## Failure handling

- Missing/locked Keychain: fail closed and ask for Electric sign-in after the
  Keychain is available.
- Expired/revoked OAuth or desktop session: keep relay access disabled.
- Wrong company, membership, public key, community, access revision, replayed
  challenge, malformed envelope, or decryption failure: reject without
  replacing the current identity.
- Keychain write/read-back failure: do not create a plaintext fallback and do
  not remove a legacy identity file.
- Logout: revoke the desktop session and managed access only. Preserve the
  native recipient identity and offline DM continuity.

Never copy sessions, private keys, device private keys, data keys, envelopes, or
live customer records into logs, screenshots, analytics, GitHub, or evidence.

## Gate sequence

1. Focused native auth, Keychain, envelope, renderer-surface, and backend
   contract tests.
2. Exact-head Hive CI and one independent identity/security review.
3. Merge the bounded client PR.
4. Deploy the already-reviewed backend migration/function only at its separate
   scoped gate.
5. Build one signed cumulative Internal Canary artifact.
6. Prove independent Andrew and Benjamin OAuth recovery plus native
   channels/DMs/profile persistence, ATRIS and shared company-VM agents,
   huddles, reconnect, cross-company denial, and bounded memory.
7. Distribute the proven artifact internally. David/customer rollout requires
   a later explicit acceptance decision.
