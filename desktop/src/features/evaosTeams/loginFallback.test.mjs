import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const authGateSource = await readFile(
  new URL("./EvaosTeamsAuthGate.tsx", import.meta.url),
  "utf8",
);
const apiSource = await readFile(new URL("./api.ts", import.meta.url), "utf8");

test("managed sign-in exposes the proof-bound backup-code path only while pending", () => {
  assert.match(authGateSource, /loginPending \? \(/);
  assert.match(authGateSource, /aria-label="Hive backup code"/);
  assert.match(authGateSource, /submitEvaosTeamsLoginCode\(backupCode\)/);
  assert.match(authGateSource, /It works only\s+for this sign-in attempt\./);
  assert.match(apiSource, /submit_evaos_teams_login_code/);
});

test("a failed login refresh cannot erase the visible action error", () => {
  const refreshIndex = authGateSource.indexOf("await refresh();");
  const actionErrorIndex = authGateSource.indexOf("setError(actionMessage);");
  assert.notEqual(refreshIndex, -1);
  assert.notEqual(actionErrorIndex, -1);
  assert.ok(refreshIndex < actionErrorIndex);
});

test("lost-device identity replacement is explicit, consequential, and command-backed", () => {
  assert.match(authGateSource, /I no longer have an authorized device/);
  assert.match(authGateSource, /old key[\s\S]*loses relay access/);
  assert.match(
    authGateSource,
    /offline messages addressed only to[\s\S]*old key may not be recoverable/,
  );
  assert.match(authGateSource, /Replace identity on this Mac/);
  assert.match(authGateSource, /replaceLostEvaosTeamsIdentity/);
  assert.match(authGateSource, /working \|\| replacingLostIdentity\.current/);
  assert.match(
    authGateSource,
    /disabled=\{working \|\| recoveryWorking \|\| recoveryStarted\}/,
  );
  assert.match(
    authGateSource,
    /!recoveryCode\.trim\(\) \|\| recoveryWorking \|\| working/,
  );
  assert.match(authGateSource, /recoveryStarted \|\|\s+working/);
  assert.match(
    authGateSource,
    /async function runLogin\(\) \{\s+setLostDeviceConfirmed\(false\);/,
  );
  assert.match(apiSource, /replace_lost_evaos_teams_identity/);
});
