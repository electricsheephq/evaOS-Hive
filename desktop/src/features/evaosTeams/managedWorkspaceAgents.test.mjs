import assert from "node:assert/strict";
import test from "node:test";
import { QueryClient } from "@tanstack/react-query";

import { relayAgentIsSharedWithUser } from "../agents/lib/agentAutocompleteEligibility.ts";
import {
  invalidateManagedWorkspaceAgentProjection,
  managedWorkspaceAgentsQueryKey,
  projectManagedWorkspaceAgents,
} from "./managedWorkspaceAgents.ts";

test("projects only company-authorized agents and their assigned rooms", () => {
  const currentPubkey = "b2".repeat(32);
  const projected = projectManagedWorkspaceAgents(
    {
      role: "owner",
      accessRevision: 1,
      reconciliationStatus: "current",
      seatLimit: 10,
      activeSeats: 1,
      pendingSeats: 0,
      members: [
        {
          membershipId: "member-current",
          publicKey: currentPubkey,
          bindingStatus: "bound",
          displayName: "Current user",
          email: "current@example.test",
          role: "owner",
        },
      ],
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
          humanMembers: ["member-current"],
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
    },
    currentPubkey.toUpperCase(),
  );

  assert.deepEqual(projected, [
    {
      pubkey: "a1".repeat(32),
      name: "Atris",
      agentType: "hermes",
      channels: ["general"],
      channelIds: ["room-general"],
      capabilities: [],
      status: "offline",
      respondTo: "allowlist",
      respondToAllowlist: [currentPubkey],
    },
  ]);

  assert.equal(
    relayAgentIsSharedWithUser(
      projected[0],
      new Set(["room-general"]),
      currentPubkey,
    ),
    true,
    "an assigned company agent is invocable by the assigned signed-in member",
  );
});

test("does not invent an author grant when the signed-in member is unassigned", () => {
  const currentPubkey = "b2".repeat(32);
  const [projected] = projectManagedWorkspaceAgents(
    {
      role: "member",
      accessRevision: 1,
      reconciliationStatus: "current",
      seatLimit: 10,
      activeSeats: 2,
      pendingSeats: 0,
      members: [
        {
          membershipId: "member-current",
          publicKey: currentPubkey,
          bindingStatus: "bound",
          displayName: "Current user",
          email: "current@example.test",
          role: "member",
        },
      ],
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
          roomId: "room-private",
          name: "private",
          channelType: "stream",
          humanMembers: ["member-other"],
          agentInstances: ["agent-atris"],
        },
      ],
    },
    currentPubkey,
  );

  assert.equal(projected.respondTo, null);
  assert.deepEqual(projected.respondToAllowlist, []);
  assert.equal(
    relayAgentIsSharedWithUser(
      projected,
      new Set(["room-private"]),
      currentPubkey,
    ),
    false,
  );
});

test("add and remove assignment refresh the canonical managed agent projection", async () => {
  const queryClient = new QueryClient();
  const projectionKey = managedWorkspaceAgentsQueryKey("company:user:1");

  for (const mutation of ["add", "remove"]) {
    queryClient.setQueryData(projectionKey, { mutation });
    assert.equal(
      queryClient.getQueryState(projectionKey)?.isInvalidated,
      false,
    );

    await invalidateManagedWorkspaceAgentProjection(queryClient);

    assert.equal(
      queryClient.getQueryState(projectionKey)?.isInvalidated,
      true,
      `${mutation} invalidates the shared collaboration-backed projection`,
    );
    queryClient.removeQueries({ queryKey: projectionKey });
  }
});
