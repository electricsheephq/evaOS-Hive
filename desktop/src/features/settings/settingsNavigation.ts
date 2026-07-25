import type { EvaosTeamsAuthority } from "@/features/evaosTeams/authority";
import {
  canManageCommunityMembers,
  type RelayMembershipLookup,
} from "@/shared/api/relayMembers";

export function canShowCommunityMembersSettings(
  authority: EvaosTeamsAuthority,
  relayMembership: RelayMembershipLookup | undefined,
): boolean {
  if (authority.managed) {
    return authority.policy.canManageMembership;
  }
  return canManageCommunityMembers(relayMembership);
}
