#!/usr/bin/env node

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(repoRoot, path), "utf8");
const workflow = read(".github/workflows/evaos-teams-pre-sign.yml");
const preSign = read("scripts/evaos-teams-pre-sign-smoke.sh");
const verifier = read("scripts/verify-evaos-teams-app-bundle.mjs");
const runbook = read("docs/security/evaos-teams-0.4.23-es.1-pre-sign.md");

assert.match(workflow, /github\.repository == 'electricsheephq\/evaOS-Hive'/);
assert.match(workflow, /permissions:\s*\n\s*contents: read/);
assert.doesNotMatch(workflow, /\b(?:write|id-token):/);
assert.match(workflow, /expected_sha/);
assert.match(workflow, /github\.event\.pull_request\.head\.sha/);
assert.match(workflow, /EVAOS_TEAMS_PRE_SIGN_ISOLATED: "1"/);
assert.match(workflow, /TAURI_BUNDLER_DMG_IGNORE_CI: "true"/);
assert.match(workflow, /scripts\/evaos-teams-pre-sign-smoke\.sh/);
assert.match(workflow, /runs-on: macos-latest/);
assert.match(workflow, /pull_request:\s*\n\s*workflow_dispatch:/);
assert.doesNotMatch(workflow, /\bpaths:/);
assert.match(workflow, /cancel-in-progress: true/);

for (const forbidden of [
  /actions\/upload-artifact/,
  /\bsecrets\./,
  /\bxcrun\s+(?:notarytool|stapler)\b/,
  /\bcodesign\s+(?:--sign|-s)\b/,
  /\bsecurity\s+import\b/,
  /\bgh\s+release\b/,
  /\baws\s+s3\b/,
  /\bblock\/apple-codesign-action\b/,
]) {
  assert.doesNotMatch(workflow, forbidden);
}

assert.match(preSign, /git rev-parse HEAD/);
assert.match(preSign, /git status --porcelain/);
assert.match(preSign, /uname -m/);
assert.match(preSign, /aarch64-apple-darwin/);
assert.match(preSign, /bundle-sidecars\.sh/);
assert.match(preSign, /run tauri:build:evaos-teams --no-sign/);
assert.doesNotMatch(preSign, /tauri:build:evaos-teams -- --no-sign/);
assert.match(preSign, /verify-evaos-teams-app-bundle\.mjs/);
assert.match(preSign, /CLEAN_ENV=/);
assert.match(preSign, /env -i "\$\{CLEAN_ENV\[@\]\}" cargo build/);
assert.match(preSign, /env -i "\$\{CLEAN_ENV\[@\]\}" node/);
assert.match(preSign, /rm -rf -- "\$APP_PATH"/);
assert.match(preSign, /rm -f -- "\$DEFAULT_DMG_PATH" "\$DMG_PATH"/);
assert.match(preSign, /BUZZ_UPDATER_PUBLIC_KEY/);
assert.match(preSign, /APPLE_SIGNING_IDENTITY/);
assert.match(preSign, /TAURI_SIGNING_PRIVATE_KEY/);

assert.match(verifier, /CFBundleIdentifier/);
assert.match(verifier, /CFBundleShortVersionString/);
assert.match(verifier, /CFBundleURLTypes\.0\.CFBundleURLSchemes\.0/);
assert.match(verifier, /buzz-acp sidecar/);
assert.match(verifier, /git-credential-nostr sidecar/);
assert.match(verifier, /requireArm64/);
assert.match(verifier, /mkdtempSync/);
assert.match(verifier, /environmentPolicy: "explicit allowlist"/);
assert.match(verifier, /updaterEndpoints: 0/);
assert.match(
  verifier,
  /packagedNetworkObservation: "not performed before signing"/,
);

for (const requiredBoundary of [
  "Cold signed-out launch",
  "Explicit login",
  "Active managed session",
  "No pre-sign evidence is signing",
  "prepared signing Mac",
]) {
  assert.match(runbook, new RegExp(requiredBoundary));
}

console.log("evaOS Teams credential-free pre-sign contract passed");
