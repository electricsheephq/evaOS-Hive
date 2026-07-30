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

test("unmanaged native identity import remains available upstream", () => {
  assert.match(nativeImportForm, /Private key/);
  assert.match(nativeImportForm, /nsec/i);
});
