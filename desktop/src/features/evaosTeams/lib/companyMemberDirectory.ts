import type { HiveCompanyMember } from "@/features/evaosTeams/api";
import type { UserSearchResult } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export function companyMemberPubkeys(
  members: readonly HiveCompanyMember[],
): ReadonlySet<string> {
  return new Set(members.map((member) => normalizePubkey(member.publicKey)));
}

export function isManagedDirectoryCandidate({
  candidate,
  managed,
  memberPubkeys,
}: {
  candidate: Pick<UserSearchResult, "isAgent" | "pubkey">;
  managed: boolean;
  memberPubkeys: ReadonlySet<string>;
}) {
  return (
    !managed ||
    candidate.isAgent ||
    memberPubkeys.has(normalizePubkey(candidate.pubkey))
  );
}

export function companyMemberAsSearchResult(
  member: HiveCompanyMember,
): UserSearchResult {
  return {
    pubkey: normalizePubkey(member.publicKey),
    displayName: member.displayName,
    avatarUrl: null,
    nip05Handle: null,
    ownerPubkey: null,
    isAgent: false,
  };
}

export function mergeCompanyDirectorySearchResults({
  managed,
  members,
  relayUsers,
}: {
  managed: boolean;
  members: readonly HiveCompanyMember[];
  relayUsers: readonly UserSearchResult[];
}): UserSearchResult[] {
  if (!managed) {
    return [...relayUsers];
  }
  const memberPubkeys = companyMemberPubkeys(members);
  return [
    ...members.map(companyMemberAsSearchResult),
    ...relayUsers.filter((candidate) =>
      isManagedDirectoryCandidate({
        candidate,
        managed,
        memberPubkeys,
      }),
    ),
  ];
}
