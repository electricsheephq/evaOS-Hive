import assert from "node:assert/strict";
import test from "node:test";

import { buildUnifiedGroups } from "./unifiedAgentGroups.ts";

const welcomePersona = {
  id: "builtin:fizz",
  displayName: "TARS",
};
const customPersona = {
  id: "custom:researcher",
  displayName: "Researcher",
};
const welcomeAgent = {
  name: "TARS",
  personaId: "builtin:fizz",
  pubkey: "welcome-agent",
  status: "stopped",
  teamId: "builtin-team:welcome",
};
const customAgent = {
  name: "Researcher",
  personaId: "custom:researcher",
  pubkey: "custom-agent",
  status: "stopped",
  teamId: "custom-team:research",
};

test("native grouping keeps starter and custom agents together in Hive", () => {
  const result = buildUnifiedGroups(
    [welcomePersona, customPersona],
    [welcomeAgent, customAgent],
  );

  assert.deepEqual(
    result.groups.map(({ persona }) => persona.id),
    ["builtin:fizz", "custom:researcher"],
  );
  assert.deepEqual(
    result.groups.flatMap(({ agents }) => agents.map(({ pubkey }) => pubkey)),
    ["welcome-agent", "custom-agent"],
  );
});

test("native grouping preserves orphaned records for normal recovery surfaces", () => {
  const result = buildUnifiedGroups(
    [],
    [
      {
        ...welcomeAgent,
        personaId: "missing:persona",
      },
      {
        ...welcomeAgent,
        personaId: null,
      },
    ],
  );

  assert.equal(result.unknown.length, 1);
  assert.equal(result.ungrouped.length, 1);
});
