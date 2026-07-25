import type { QueryClient } from "@tanstack/react-query";
import type { RelayAgent } from "@/shared/api/types";

import { hiveCollaborationQueryKey, type HiveCollaborationState } from "./api";

export function managedWorkspaceAgentsQueryKey(authorityCacheKey: string) {
  return [...hiveCollaborationQueryKey, authorityCacheKey] as const;
}

export function invalidateManagedWorkspaceAgentProjection(
  queryClient: Pick<QueryClient, "invalidateQueries">,
) {
  return queryClient.invalidateQueries({
    queryKey: hiveCollaborationQueryKey,
  });
}

/**
 * Projects the company-authorized Hermes catalog into the upstream relay-agent
 * shape used by mentions, member classification, and activity surfaces.
 * Managed Hive must not fall back to laptop-local agent discovery.
 */
export function projectManagedWorkspaceAgents(
  state: HiveCollaborationState,
  currentPubkey?: string | null,
): RelayAgent[] {
  const normalizedCurrentPubkey = currentPubkey?.trim().toLowerCase() || null;
  const currentMembership = normalizedCurrentPubkey
    ? state.members.find(
        (member) =>
          member.publicKey?.trim().toLowerCase() === normalizedCurrentPubkey,
      )
    : undefined;

  return state.agents.map((agent) => {
    const rooms = state.rooms.filter((room) =>
      room.agentInstances.includes(agent.agentInstanceId),
    );
    const currentUserMayInvoke =
      currentMembership !== undefined &&
      rooms.some((room) =>
        room.humanMembers.includes(currentMembership.membershipId),
      );

    return {
      pubkey: agent.publicKey,
      name: agent.displayName,
      agentType: agent.runtime,
      channels: rooms.map((room) => room.name ?? room.roomId),
      channelIds: rooms.map((room) => room.roomId),
      capabilities: [],
      status: "offline",
      // The managed collaboration projection proves both halves of the VM
      // gate: this agent and the signed-in member share an assigned room.
      // Never widen that server-derived grant to `anyone`.
      respondTo: currentUserMayInvoke ? "allowlist" : null,
      respondToAllowlist:
        currentUserMayInvoke && normalizedCurrentPubkey
          ? [normalizedCurrentPubkey]
          : [],
    };
  });
}
