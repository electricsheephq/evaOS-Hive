# Hive 0.4.26-es.1 pre-sign gate

This gate proves that an exact clean source SHA can produce the managed Apple
Silicon app and DMG without credentials. It cannot sign, notarize, staple,
upload, publish, install, or mutate a customer or runtime.

## Credential-free proof

Run `scripts/evaos-teams-pre-sign-smoke.sh` with the exact source SHA and an
absolute evidence path. The workflow must use an explicit environment
allowlist, reject signing and updater credentials, build every arm64 sidecar,
verify the package contract, and keep a new empty-home signed-out launch alive.
No pre-sign evidence is signing, notarization, installation, distribution, or
runtime proof.

## Packaged network observation after signing

- **Cold signed-out launch:** no external request is allowed.
- **Explicit login:** only Electric Sheep login, the loopback callback, and the
  baked Supabase Functions origin are allowed.
- **Active managed session:** those destinations plus the server-selected relay
  are allowed.
- **Every phase:** updater, Buzz, Block, GitHub, and other upstream hosts must
  remain at zero requests.

Record the exact source SHA, signed app and DMG hashes, signer identity,
notarization request, stapling result, and observed destinations by phase.

## Prepared-Mac handoff

The signing owner must name the prepared signing Mac and immutable private
artifact store before credentials are loaded. Signing, notarization, stapling,
installation, installed-app smoke, checksum, private upload, and dashboard
allowlisting remain separate gates.
