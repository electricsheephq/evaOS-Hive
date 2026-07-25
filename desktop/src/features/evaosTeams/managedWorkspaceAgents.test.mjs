import assert from "node:assert/strict";
import test from "node:test";

import { projectManagedWorkspaceAgents } from "./managedWorkspaceAgents.ts";

test("projects only company-authorized agents and their assigned rooms", () => {
  const projected = projectManagedWorkspaceAgents({
    role: "owner",
    accessRevision: 1,
    reconciliationStatus: "current",
    seatLimit: 10,
    activeSeats: 1,
    pendingSeats: 0,
    members: [],
    agents: [
      {
        agentInstanceId: "agent-atris",
        publicKey: "a1".repeat(32),
        displayName: "Atris",
        runtime: "hermes",
      },
    ],
    rooms: [
      {
        roomId: "room-general",
        name: "general",
        channelType: "stream",
        humanMembers: [],
        agentInstances: ["agent-atris"],
      },
      {
        roomId: "room-other",
        name: "other",
        channelType: "stream",
        humanMembers: [],
        agentInstances: [],
      },
    ],
  });

  assert.deepEqual(projected, [
    {
      pubkey: "a1".repeat(32),
      name: "Atris",
      agentType: "hermes",
      channels: ["general"],
      channelIds: ["room-general"],
      capabilities: [],
      status: "offline",
      respondTo: null,
      respondToAllowlist: [],
    },
  ]);
});
