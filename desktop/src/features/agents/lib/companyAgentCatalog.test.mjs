import assert from "node:assert/strict";
import test from "node:test";

import {
  filterBuiltinWelcomeAgents,
  filterBuiltinWelcomePersonas,
  filterBuiltinWelcomeTeams,
  hasCanonicalCompanyWelcomeAgents,
  intersectAuthorizedCompanyAgents,
  resolveCompanyVmAgents,
} from "./companyAgentCatalog.ts";

const PUBKEYS = {
  tars: "a".repeat(64),
  samantha: "b".repeat(64),
  hal: "c".repeat(64),
  atris: "d".repeat(64),
  foreign: "e".repeat(64),
};

function relayAgent(pubkey, name, overrides = {}) {
  return {
    pubkey,
    name,
    agentType: "general",
    channels: ["general"],
    channelIds: ["channel-general"],
    capabilities: ["chat"],
    status: "online",
    respondTo: "allowlist",
    respondToAllowlist: ["f".repeat(64)],
    ...overrides,
  };
}

function authorization(publicKey, agentId, runtime = "hermes") {
  return { publicKey, agentId, runtime };
}

test("intersects catalog authorization with native relay identities", () => {
  const native = relayAgent(PUBKEYS.tars, "Native TARS", {
    channels: ["private"],
    status: "away",
  });
  const result = intersectAuthorizedCompanyAgents(
    [native, relayAgent(PUBKEYS.foreign, "Other company")],
    [
      authorization(PUBKEYS.tars.toUpperCase(), "tars"),
      authorization(PUBKEYS.atris, "atris"),
    ],
  );

  assert.deepEqual(result, [
    {
      ...native,
      agentId: "tars",
      runtime: "hermes",
    },
  ]);
});

test("never synthesizes a catalog-only company agent", () => {
  assert.deepEqual(
    intersectAuthorizedCompanyAgents(
      [],
      [authorization(PUBKEYS.atris, "atris")],
    ),
    [],
  );
});

test("failed catalog or relay refresh drops stale company authorization", () => {
  const relayAgents = [relayAgent(PUBKEYS.tars, "Native TARS")];
  const authorizations = [authorization(PUBKEYS.tars, "tars")];
  assert.equal(
    resolveCompanyVmAgents(relayAgents, authorizations, false).length,
    1,
  );
  assert.deepEqual(
    resolveCompanyVmAgents(relayAgents, authorizations, true),
    [],
  );
});

test("drops invalid and duplicate authorization records", () => {
  assert.deepEqual(
    intersectAuthorizedCompanyAgents(
      [relayAgent(PUBKEYS.tars, "Native TARS")],
      [
        authorization("not-a-key", "invalid"),
        authorization(PUBKEYS.tars, "tars"),
        authorization(PUBKEYS.tars, "duplicate"),
        authorization(PUBKEYS.foreign, "foreign", " "),
      ],
    ).map(({ agentId }) => agentId),
    ["tars"],
  );
});

test("suppresses built-in welcome presentation only after all canonical VM identities exist", () => {
  const incomplete = [
    {
      ...relayAgent(PUBKEYS.tars, "TARS"),
      agentId: "tars",
      runtime: "hermes",
    },
  ];
  const complete = [
    ...incomplete,
    {
      ...relayAgent(PUBKEYS.samantha, "Samantha"),
      agentId: "samantha",
      runtime: "hermes",
    },
    {
      ...relayAgent(PUBKEYS.hal, "HAL 9000"),
      agentId: "hal-9000",
      runtime: "hermes",
    },
  ];

  assert.equal(hasCanonicalCompanyWelcomeAgents(incomplete), false);
  assert.equal(hasCanonicalCompanyWelcomeAgents(complete), true);
});

test("managed presentation filtering preserves custom local records", () => {
  const personas = [{ id: "builtin:fizz" }, { id: "custom:tars" }];
  const agents = [
    { pubkey: PUBKEYS.tars, personaId: "builtin:fizz", teamId: null },
    { pubkey: PUBKEYS.atris, personaId: "custom:tars", teamId: null },
    {
      pubkey: PUBKEYS.foreign,
      personaId: "custom:welcome-member",
      teamId: "builtin-team:welcome",
    },
  ];
  const teams = [{ id: "builtin-team:welcome" }, { id: "custom-team" }];

  assert.deepEqual(
    filterBuiltinWelcomePersonas(personas, true).map(({ id }) => id),
    ["custom:tars"],
  );
  assert.deepEqual(
    filterBuiltinWelcomeAgents(agents, true).map(({ pubkey }) => pubkey),
    [PUBKEYS.atris, PUBKEYS.foreign],
  );
  assert.deepEqual(
    filterBuiltinWelcomeTeams(teams, true).map(({ id }) => id),
    ["custom-team"],
  );
  assert.equal(filterBuiltinWelcomePersonas(personas, false), personas);
  assert.equal(filterBuiltinWelcomeAgents(agents, false), agents);
  assert.equal(filterBuiltinWelcomeTeams(teams, false), teams);
});
