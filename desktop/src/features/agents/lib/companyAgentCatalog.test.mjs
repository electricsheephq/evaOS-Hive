import assert from "node:assert/strict";
import test from "node:test";

import {
  companyVmAgentsFromCatalog,
  mergeRelayAgentsWithCompanyCatalog,
  resolveCompanyAgentPresenceStatuses,
  resolveCompanyAgentVisibleChannels,
  resolveRelayAgentProfiles,
} from "./companyAgentCatalog.ts";

const PUBKEY = "a".repeat(64);
const COMPANY_AGENT_PUBKEYS = new Set([PUBKEY]);

test("company catalog adds a VM agent with unknown liveness", () => {
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
      status: "unknown",
      respondTo: null,
      respondToAllowlist: [],
    },
  ]);
});

test("valid signed relay name and live status win over catalog fallback", () => {
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
  assert.equal(merged[0].name, "old name");
  assert.equal(merged[0].agentType, "hermes");
  assert.equal(merged[0].status, "online");
  assert.deepEqual(merged[0].channels, ["general"]);
  assert.deepEqual(merged[0].capabilities, ["chat", "company-vm", "hermes"]);
});

test("raw relay names fall back to the company display name", () => {
  const merged = mergeRelayAgentsWithCompanyCatalog(
    [
      {
        pubkey: PUBKEY,
        name: PUBKEY,
        agentType: "unknown",
        channels: [],
        channelIds: [],
        capabilities: [],
        status: "offline",
        respondTo: null,
        respondToAllowlist: [],
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
  assert.equal(merged[0].name, "ATRIS");
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
        name: PUBKEY,
      },
    ],
    companyCatalog,
  );
  assert.equal(selected.length, 1);
  assert.equal(selected[0].pubkey, PUBKEY);
  assert.equal(selected[0].name, "ATRIS");
});

test("signed native profile name wins consistently after catalog merge", () => {
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
  const resolved = resolveRelayAgentProfiles(
    merged,
    {
      [PUBKEY]: {
        displayName: "Atris Signed",
        name: null,
        avatarUrl: "https://example.test/atris.png",
        nip05Handle: null,
        ownerPubkey: null,
        isAgent: true,
      },
    },
    COMPANY_AGENT_PUBKEYS,
  );
  assert.equal(resolved?.[0].name, "Atris Signed");
});

test("company agent channels come only from actual visible native membership", () => {
  const agent = mergeRelayAgentsWithCompanyCatalog(
    [
      {
        pubkey: PUBKEY,
        name: "ATRIS",
        agentType: "hermes",
        channels: ["stale-private-room"],
        channelIds: ["stale-room"],
        capabilities: [],
        status: "online",
        respondTo: null,
        respondToAllowlist: [],
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
  const resolved = resolveCompanyAgentVisibleChannels(
    agent,
    [
      {
        id: "visible-room",
        name: "general",
        channelType: "stream",
        visibility: "open",
        description: "",
        topic: null,
        purpose: null,
        memberCount: 1,
        memberPubkeys: [PUBKEY],
        lastMessageAt: null,
        archivedAt: null,
        participants: [],
        participantPubkeys: [],
        isMember: true,
        ttlSeconds: null,
        ttlDeadline: null,
      },
      {
        id: "not-a-member",
        name: "other",
        channelType: "stream",
        visibility: "open",
        description: "",
        topic: null,
        purpose: null,
        memberCount: 1,
        memberPubkeys: ["b".repeat(64)],
        lastMessageAt: null,
        archivedAt: null,
        participants: [],
        participantPubkeys: [],
        isMember: false,
        ttlSeconds: null,
        ttlDeadline: null,
      },
    ],
    COMPANY_AGENT_PUBKEYS,
  );
  assert.deepEqual(resolved?.[0].channels, ["general"]);
  assert.deepEqual(resolved?.[0].channelIds, ["visible-room"]);
});

test("company agent channels stay empty until native membership resolves", () => {
  const companyAgent = {
    pubkey: PUBKEY,
    name: "ATRIS",
    agentType: "hermes",
    channels: ["claimed-private-room"],
    channelIds: ["claimed-room"],
    capabilities: ["company-vm"],
    status: "offline",
    respondTo: null,
    respondToAllowlist: [],
  };
  const resolved = resolveCompanyAgentVisibleChannels(
    [companyAgent],
    undefined,
    COMPANY_AGENT_PUBKEYS,
  );
  assert.deepEqual(resolved?.[0].channels, []);
  assert.deepEqual(resolved?.[0].channelIds, []);
});

test("self-declared company capability cannot trigger catalog-only transforms", () => {
  const relayOnlyPubkey = "b".repeat(64);
  const relayOnly = {
    pubkey: relayOnlyPubkey,
    name: "Relay only",
    agentType: "unknown",
    channels: ["signed-room"],
    channelIds: ["signed-room-id"],
    capabilities: ["company-vm"],
    status: "online",
    respondTo: null,
    respondToAllowlist: [],
  };
  const resolved = resolveCompanyAgentVisibleChannels(
    [relayOnly],
    [],
    COMPANY_AGENT_PUBKEYS,
  );
  assert.strictEqual(resolved?.[0], relayOnly);
});

test("company agent liveness comes only from presence", () => {
  const companyAgent = {
    pubkey: PUBKEY,
    name: "ATRIS",
    agentType: "hermes",
    channels: [],
    channelIds: [],
    capabilities: ["company-vm"],
    status: "offline",
    respondTo: null,
    respondToAllowlist: [],
  };
  const unresolved = resolveCompanyAgentPresenceStatuses(
    [companyAgent],
    undefined,
    COMPANY_AGENT_PUBKEYS,
  );
  assert.equal(unresolved?.[0].status, "unknown");

  const online = resolveCompanyAgentPresenceStatuses(
    [companyAgent],
    { [PUBKEY]: "online" },
    COMPANY_AGENT_PUBKEYS,
  );
  assert.equal(online?.[0].status, "online");

  const away = resolveCompanyAgentPresenceStatuses(
    [companyAgent],
    { [PUBKEY]: "away" },
    COMPANY_AGENT_PUBKEYS,
  );
  assert.equal(away?.[0].status, "unknown");
});
