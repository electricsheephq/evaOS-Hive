import assert from "node:assert/strict";
import test from "node:test";

import { mergeRelayAgentsWithCompanyCatalog } from "./companyAgentCatalog.ts";

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
  assert.deepEqual(merged[0].channels, ["general"]);
  assert.deepEqual(merged[0].capabilities, ["chat", "company-vm", "hermes"]);
});
