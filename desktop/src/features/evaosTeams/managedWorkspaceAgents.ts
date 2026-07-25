import type { RelayAgent } from "@/shared/api/types";

import type { HiveCollaborationState } from "./api";

/**
 * Projects the company-authorized Hermes catalog into the upstream relay-agent
 * shape used by mentions, member classification, and activity surfaces.
 * Managed Hive must not fall back to laptop-local agent discovery.
 */
export function projectManagedWorkspaceAgents(
  state: HiveCollaborationState,
): RelayAgent[] {
  return state.agents.map((agent) => {
    const rooms = state.rooms.filter((room) =>
      room.agentInstances.includes(agent.agentInstanceId),
    );

    return {
      pubkey: agent.publicKey,
      name: agent.displayName,
      agentType: agent.runtime,
      channels: rooms.map((room) => room.name ?? room.roomId),
      channelIds: rooms.map((room) => room.roomId),
      capabilities: [],
      status: "offline",
      respondTo: null,
      respondToAllowlist: [],
    };
  });
}
