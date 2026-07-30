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
