import { createContext, type ReactNode, useContext } from "react";

import type { Community } from "@/features/communities/types";
import type { EvaosTeamsEntitlement } from "@/features/evaosTeams/api";

export type EvaosTeamsRole =
  | "owner"
  | "admin"
  | "member"
  | "employee"
  | "agent_only";

export type EvaosTeamsAuthorityPolicy = {
  canManageCommunities: boolean;
  canManageMembership: boolean;
  canManageChannels: boolean;
  canManageAgents: boolean;
  canBrowsePeople: boolean;
  canBrowseAgents: boolean;
  canBrowsePrivateRooms: boolean;
  canStartDirectMessages: boolean;
  canViewMembers: boolean;
};

export type EvaosTeamsAuthority = {
  managed: boolean;
  cacheKey: string;
  entitlement?: EvaosTeamsEntitlement;
  community?: Community;
  policy: EvaosTeamsAuthorityPolicy;
};

const nativePolicy: EvaosTeamsAuthorityPolicy = {
  canManageCommunities: true,
  canManageMembership: true,
  canManageChannels: true,
  canManageAgents: true,
  canBrowsePeople: true,
  canBrowseAgents: true,
  canBrowsePrivateRooms: true,
  canStartDirectMessages: true,
  canViewMembers: true,
};

export const nativeEvaosTeamsAuthority: EvaosTeamsAuthority = {
  managed: false,
  cacheKey: "native",
  policy: nativePolicy,
};

export function managedAuthorityPolicy(
  role: EvaosTeamsRole,
): EvaosTeamsAuthorityPolicy {
  const agentOnly = role === "agent_only";
  const managesWorkspace = role === "owner" || role === "admin";
  return {
    canManageCommunities: false,
    // Managed membership and agent mutations use the dedicated workspace
    // settings broker, never Buzz's native relay/local-agent commands.
    canManageMembership: managesWorkspace,
    canManageChannels: !agentOnly,
    canManageAgents: false,
    canBrowsePeople: !agentOnly,
    // The native Agents screen controls local runtimes and is not meaningful
    // in a company-managed workspace. Company agents remain visible and
    // assignable through the brokered Workspace access panel.
    canBrowseAgents: false,
    canBrowsePrivateRooms: !agentOnly,
    canStartDirectMessages: !agentOnly,
    canViewMembers: !agentOnly,
  };
}

export function managedCommunityFromEntitlement(
  entitlement: EvaosTeamsEntitlement,
): Community {
  const relay = new URL(entitlement.relayHost);
  relay.protocol = "wss:";
  return {
    id: entitlement.communityId,
    name: "Hive",
    relayUrl: relay.toString().replace(/\/$/, ""),
    pubkey: entitlement.publicKey,
    addedAt: entitlement.expiresAt,
  };
}

export function createManagedEvaosTeamsAuthority(
  entitlement: EvaosTeamsEntitlement,
): EvaosTeamsAuthority {
  return {
    managed: true,
    cacheKey: [
      entitlement.communityId,
      entitlement.publicKey ?? "unbound",
      entitlement.accessRevision,
    ].join(":"),
    entitlement,
    community: managedCommunityFromEntitlement(entitlement),
    policy: managedAuthorityPolicy(entitlement.role),
  };
}

const EvaosTeamsAuthorityContext = createContext<EvaosTeamsAuthority>(
  nativeEvaosTeamsAuthority,
);

export function EvaosTeamsAuthorityProvider({
  authority,
  children,
}: {
  authority: EvaosTeamsAuthority;
  children: ReactNode;
}) {
  return (
    <EvaosTeamsAuthorityContext.Provider value={authority}>
      {children}
    </EvaosTeamsAuthorityContext.Provider>
  );
}

export function useEvaosTeamsAuthority() {
  return useContext(EvaosTeamsAuthorityContext);
}

export function canSearchEvaosTeamsAuthority(
  policy: EvaosTeamsAuthorityPolicy,
) {
  return (
    policy.canBrowsePeople ||
    policy.canBrowseAgents ||
    policy.canBrowsePrivateRooms
  );
}

export function isEvaosTeamsViewAllowed(
  view: string,
  policy: EvaosTeamsAuthorityPolicy,
) {
  return (
    (view !== "agents" || policy.canBrowseAgents) &&
    (view !== "messages" || policy.canStartDirectMessages)
  );
}

export function requireEvaosTeamsAuthority(allowed: boolean, message: string) {
  if (!allowed) throw new Error(message);
}
