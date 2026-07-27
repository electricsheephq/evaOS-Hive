import assert from "node:assert/strict";
import test from "node:test";

import {
  buildUnifiedGroups,
  filterManagedLocalWelcomeRecords,
} from "./unifiedAgentGroups.ts";

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

test("managed Hive hides renamed built-in welcome records without deleting custom agents", () => {
  const result = filterManagedLocalWelcomeRecords(
    buildUnifiedGroups(
      [welcomePersona, customPersona],
      [welcomeAgent, customAgent],
    ),
    true,
  );

  assert.deepEqual(
    result.groups.map(({ persona }) => persona.id),
    ["custom:researcher"],
  );
  assert.deepEqual(
    result.groups.flatMap(({ agents }) => agents.map(({ pubkey }) => pubkey)),
    ["custom-agent"],
  );
});

test("managed Hive hides orphaned welcome-team records", () => {
  const result = filterManagedLocalWelcomeRecords(
    buildUnifiedGroups(
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
    ),
    true,
  );

  assert.deepEqual(result.unknown, []);
  assert.deepEqual(result.ungrouped, []);
});

test("unmanaged Buzz keeps native welcome records unchanged", () => {
  const unifiedGroups = buildUnifiedGroups([welcomePersona], [welcomeAgent]);

  assert.strictEqual(
    filterManagedLocalWelcomeRecords(unifiedGroups, false),
    unifiedGroups,
  );
});
