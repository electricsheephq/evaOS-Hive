import assert from "node:assert/strict";
import test from "node:test";

import {
  excludeLocalPubkeyDuplicates,
  filterLocalWelcomeAgents,
  filterLocalWelcomePersonas,
  filterLocalWelcomeTeams,
  hasCanonicalCompanyWelcomeAgents,
  mergeCompanyAgentsWithRelay,
} from "./companyAgentCatalog.ts";

const PUBKEY = "a".repeat(64);
const INSTANCE = "10000000-0000-4000-8000-000000000001";

function companyAgent(overrides = {}) {
  return {
    agentInstanceId: INSTANCE,
    agentId: "tars",
    publicKey: PUBKEY,
    displayName: "TARS catalog",
    runtime: "hermes",
    ...overrides,
  };
}

test("catalog-only records have unknown status and no collaboration authority", () => {
  assert.deepEqual(mergeCompanyAgentsWithRelay([], [companyAgent()]), [
    {
      agentInstanceId: INSTANCE,
      agentId: "tars",
      pubkey: PUBKEY,
      name: "TARS catalog",
      agentType: "hermes",
      runtime: "hermes",
      channels: [],
      channelIds: [],
      capabilities: [],
      status: "unknown",
      respondTo: null,
      respondToAllowlist: [],
    },
  ]);
});

test("signed relay collaboration fields win over catalog fallback", () => {
  const relayAgent = {
    pubkey: PUBKEY.toUpperCase(),
    name: "TARS signed",
    agentType: "acp",
    channels: ["general"],
    channelIds: ["room-1"],
    capabilities: ["chat"],
    status: "online",
    respondTo: "allowlist",
    respondToAllowlist: ["b".repeat(64)],
  };
  const [merged] = mergeCompanyAgentsWithRelay([relayAgent], [companyAgent()]);
  assert.deepEqual(merged, {
    agentInstanceId: INSTANCE,
    agentId: "tars",
    pubkey: PUBKEY,
    name: "TARS signed",
    agentType: "acp",
    runtime: "hermes",
    channels: ["general"],
    channelIds: ["room-1"],
    capabilities: ["chat"],
    status: "online",
    respondTo: "allowlist",
    respondToAllowlist: ["b".repeat(64)],
  });
});

test("company records dedupe by normalized public key", () => {
  const merged = mergeCompanyAgentsWithRelay(
    [],
    [
      companyAgent(),
      companyAgent({
        publicKey: PUBKEY.toUpperCase(),
        displayName: "duplicate",
      }),
    ],
  );
  assert.equal(merged.length, 1);
  assert.equal(merged[0].name, "TARS catalog");
});

test("company cards suppress exact local public-key duplicates only", () => {
  const visible = excludeLocalPubkeyDuplicates(
    [
      { pubkey: PUBKEY.toUpperCase(), name: "same identity" },
      { pubkey: "b".repeat(64), name: "same display name" },
    ],
    [{ pubkey: PUBKEY }],
  );
  assert.deepEqual(visible, [
    { pubkey: "b".repeat(64), name: "same display name" },
  ]);
});

test("welcome suppression requires the complete canonical Hermes set", () => {
  const partial = [
    companyAgent({ agentId: "tars" }),
    companyAgent({ agentId: "samantha", publicKey: "b".repeat(64) }),
  ];
  assert.equal(hasCanonicalCompanyWelcomeAgents(partial), false);
  assert.equal(
    hasCanonicalCompanyWelcomeAgents([
      ...partial,
      companyAgent({
        agentId: "hal-9000",
        publicKey: "c".repeat(64),
        runtime: "openclaw",
      }),
    ]),
    false,
  );
  assert.equal(
    hasCanonicalCompanyWelcomeAgents([
      ...partial,
      companyAgent({
        agentId: "hal-9000",
        publicKey: "c".repeat(64),
      }),
    ]),
    true,
  );
});

test("welcome suppression uses exact stable IDs and preserves custom records", () => {
  const agents = [
    { name: "renamed", personaId: "builtin:fizz", teamId: null },
    { name: "custom Fizz", personaId: "custom:fizz", teamId: null },
    {
      name: "welcome custom",
      personaId: "custom:any",
      teamId: "builtin-team:welcome",
    },
  ];
  assert.deepEqual(filterLocalWelcomeAgents(true, agents), [agents[1]]);
  assert.deepEqual(
    filterLocalWelcomePersonas(true, [
      { id: "builtin:honey", displayName: "renamed" },
      { id: "custom:honey", displayName: "Honey" },
    ]),
    [{ id: "custom:honey", displayName: "Honey" }],
  );
  assert.deepEqual(
    filterLocalWelcomeTeams(true, [
      { id: "builtin-team:welcome", name: "renamed" },
      { id: "custom:welcome", name: "Welcome" },
    ]),
    [{ id: "custom:welcome", name: "Welcome" }],
  );
});

test("disabled suppression leaves unmanaged Buzz unchanged", () => {
  const agents = [
    { personaId: "builtin:fizz", teamId: "builtin-team:welcome" },
  ];
  assert.deepEqual(filterLocalWelcomeAgents(false, agents), agents);
});
