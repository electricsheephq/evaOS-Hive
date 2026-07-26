import assert from "node:assert/strict";
import test from "node:test";

import {
  companyVmAgentsFromCatalog,
  getRelayPolicyAgentPubkeys,
  mergeRelayAgentsWithCompanyCatalog,
} from "./companyAgentCatalog.ts";

const PUBKEY = "a".repeat(64);

test("company catalog adds an offline VM agent without claiming liveness", () => {
  const merged = mergeRelayAgentsWithCompanyCatalog(
    [],
    [
      {
        agentInstanceId: "10000000-0000-4000-8000-000000000001",
        publicKey: PUBKEY,
        displayName: "ATRIS",
        runtime: "hermes",
      },
    ],
  );
  assert.deepEqual(merged, [
    {
      pubkey: PUBKEY,
      name: "ATRIS",
      agentType: "hermes",
      channels: [],
      channelIds: [],
      capabilities: ["company-vm", "hermes"],
      status: "offline",
      respondTo: null,
      respondToAllowlist: [],
      directorySource: "company-catalog",
    },
  ]);
});

test("relay profile supplies live status while company catalog owns name and runtime", () => {
  const merged = mergeRelayAgentsWithCompanyCatalog(
    [
      {
        pubkey: PUBKEY,
        name: "old name",
        agentType: "unknown",
        channels: ["general"],
        channelIds: ["room-1"],
        capabilities: ["chat"],
        status: "online",
        respondTo: "allowlist",
        respondToAllowlist: ["b".repeat(64)],
      },
    ],
    [
      {
        agentInstanceId: "10000000-0000-4000-8000-000000000001",
        publicKey: PUBKEY,
        displayName: "ATRIS",
        runtime: "hermes",
      },
    ],
  );
  assert.equal(merged.length, 1);
  assert.equal(merged[0].name, "ATRIS");
  assert.equal(merged[0].agentType, "hermes");
  assert.equal(merged[0].status, "online");
  assert.equal(merged[0].directorySource, "relay");
  assert.deepEqual(merged[0].channels, ["general"]);
  assert.deepEqual(merged[0].capabilities, ["chat", "company-vm", "hermes"]);
});

test("relay policy provenance excludes catalog-only identities", () => {
  const catalogOnlyPubkey = "c".repeat(64);
  const relayPubkey = "d".repeat(64);
  const merged = mergeRelayAgentsWithCompanyCatalog(
    [
      {
        pubkey: relayPubkey,
        name: "Relay agent",
        agentType: "hermes",
        channels: ["general"],
        channelIds: ["room-1"],
        capabilities: ["chat"],
        status: "online",
        respondTo: "anyone",
        respondToAllowlist: [],
      },
    ],
    [
      {
        agentInstanceId: "10000000-0000-4000-8000-000000000001",
        publicKey: catalogOnlyPubkey,
        displayName: "Catalog only",
        runtime: "hermes",
      },
    ],
  );

  assert.deepEqual(getRelayPolicyAgentPubkeys(merged), new Set([relayPubkey]));
});

test("relay-only profiles are not labeled as registered company VM agents", () => {
  const relayOnly = {
    pubkey: "b".repeat(64),
    name: "Relay only",
    agentType: "unknown",
    channels: [],
    channelIds: [],
    capabilities: ["chat", "company-vm"],
    status: "online",
    respondTo: null,
    respondToAllowlist: [],
  };
  const companyCatalog = [
    {
      agentInstanceId: "10000000-0000-4000-8000-000000000001",
      publicKey: PUBKEY,
      displayName: "ATRIS",
      runtime: "hermes",
    },
  ];
  const selected = companyVmAgentsFromCatalog(
    [
      relayOnly,
      {
        ...relayOnly,
        pubkey: PUBKEY,
        name: "Untrusted relay name",
      },
    ],
    companyCatalog,
  );
  assert.equal(selected.length, 1);
  assert.equal(selected[0].pubkey, PUBKEY);
  assert.equal(selected[0].name, "ATRIS");
});
