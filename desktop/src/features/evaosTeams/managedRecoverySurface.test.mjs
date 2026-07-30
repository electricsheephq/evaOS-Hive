import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const authGate = readFileSync(
  new URL("./EvaosTeamsAuthGate.tsx", import.meta.url),
  "utf8",
);
const nativeImportForm = readFileSync(
  new URL("../onboarding/ui/NostrKeyImportForm.tsx", import.meta.url),
  "utf8",
);
const api = readFileSync(new URL("./api.ts", import.meta.url), "utf8");

test("managed sign-in exposes only Electric OAuth recovery", () => {
  assert.match(authGate, /Sign in with Electric Sheep/);
  for (const forbidden of [
    "NostrKeyImportForm",
    "importIdentity",
    "identity_restore_required",
    "nsec",
    "nostrpair",
    "Mobile",
  ]) {
    assert.doesNotMatch(authGate, new RegExp(forbidden, "i"), forbidden);
  }
});

test("managed recovery keeps the action error after refreshing rolled-back status", () => {
  assert.match(
    authGate,
    /const actionMessage =[\s\S]*await refresh\(\);[\s\S]*setError\(actionMessage\);/,
  );
});

test("lost identity replacement is explicit, consequential, and command backed", () => {
  assert.match(authGate, /status\?\.phase === "identity_reset_required"/);
  assert.match(authGate, /lostIdentityConfirmed/);
  assert.match(
    authGate,
    /Messages encrypted only to the old key may not be\s+recoverable/,
  );
  assert.match(authGate, /Replace lost identity on this Mac/);
  assert.match(authGate, /replaceLostEvaosTeamsIdentity\(\)/);
  assert.match(api, /replace_lost_evaos_teams_identity/);
});

test("unmanaged native identity import remains available upstream", () => {
  assert.match(nativeImportForm, /Private key/);
  assert.match(nativeImportForm, /nsec/i);
});
