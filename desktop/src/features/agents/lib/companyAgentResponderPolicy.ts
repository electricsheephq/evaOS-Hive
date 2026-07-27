import type { HiveCompanyMember } from "@/features/evaosTeams/api";
import type { Channel } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export type CompanyAgentRoomOption = Pick<
  Channel,
  "id" | "name" | "visibility"
>;

export function companyAgentRoomOptions(
  channels: readonly Channel[],
  agentPubkey: string,
): CompanyAgentRoomOption[] {
  const normalizedAgent = normalizePubkey(agentPubkey);
  return channels
    .filter(
      (channel) =>
        channel.channelType !== "dm" &&
        channel.isMember &&
        channel.memberPubkeys.some(
          (member) => normalizePubkey(member) === normalizedAgent,
        ),
    )
    .map(({ id, name, visibility }) => ({ id, name, visibility }))
    .sort((left, right) => left.name.localeCompare(right.name));
}

export function availablePolicyMembers(
  members: readonly HiveCompanyMember[],
): HiveCompanyMember[] {
  const seen = new Set<string>();
  return members
    .filter((member) => {
      if (seen.has(member.membershipId)) return false;
      seen.add(member.membershipId);
      return true;
    })
    .sort((left, right) => left.displayName.localeCompare(right.displayName));
}

export function canManageCompanyAgentPolicy(role: string | undefined) {
  return role === "owner" || role === "admin";
}
