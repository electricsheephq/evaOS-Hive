# evaOS Teams 0.4.23-es.1 pre-sign gate

This lane proves that an exact clean source SHA can produce the managed Apple
Silicon app and DMG without credentials. It is intentionally unable to sign,
notarize, staple, upload, publish, allowlist, install, or mutate a customer or
runtime.

## Credential-free proof

On an isolated Apple Silicon runner:

```sh
EVAOS_TEAMS_PRE_SIGN_ISOLATED=1 \
  scripts/evaos-teams-pre-sign-smoke.sh \
  <full-source-sha> \
  /absolute/private/path/evaos-teams-pre-sign-evidence.json
```

The script rejects a dirty or mismatched checkout, updater variables, and
common signing credentials. It reuses the managed build wrapper, builds the
real arm64 sidecars, checks the bundle identity and resources, checks every
managed executable is arm64-only, rejects an identity-signed result, and holds
an isolated signed-out cold start alive for eight seconds. The JSON evidence
contains source and artifact hashes but no application logs, credentials, app,
or DMG.

The GitHub workflow prints only that JSON into the job summary. It does not
upload the unsigned app or DMG. No pre-sign evidence is signing, notarization,
installation, private-distribution, or runtime evidence.

## Packaged network observation after signing

Perform the network observation on the prepared signing Mac only after the
exact app has a Developer ID signature, notarization acceptance, and a stapled
ticket. Start from a clean macOS user with no prior evaOS Teams session.

- **Cold signed-out launch:** no external request is allowed.
- **Explicit login:** only `https://www.electricsheephq.com`, the loopback login
  callback, and the baked Supabase Functions origin are allowed.
- **Active managed session:** the login destinations plus the
  entitlement-derived relay are allowed.
- **Every phase:** updater, Buzz, Block, GitHub, and any other upstream host must
  remain at zero requests.

Record the exact source SHA, signed app hash, DMG hash, signer identity,
notarization request, stapling result, observed destinations by phase, and
whether each request was allowed. Any unexpected destination blocks the canary.

## Prepared-Mac handoff

The signing owner must name the prepared signing Mac and immutable private
artifact store before credentials are loaded. Run signing, notarization,
stapling, DMG installation into `/Applications`, installed-app smoke, checksum,
private upload, and dashboard allowlisting as distinct recorded gates. Never
adapt the upstream Block signing workflow or use public GitHub Actions artifacts
for this internal canary.
