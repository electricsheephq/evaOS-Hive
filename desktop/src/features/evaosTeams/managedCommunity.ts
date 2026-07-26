import type { Community } from "@/features/communities/types";

import type { EvaosTeamsEntitlement } from "./api";

export type ManagedCommunityState = {
  activeId: string;
  communities: Community[];
  retiredCommunities: Community[];
};

export function forEachRetiredManagedCommunity(
  retiredCommunities: Community[],
  clearRetiredCommunity: (community: Community) => void,
): void {
  for (const community of retiredCommunities) {
    clearRetiredCommunity(community);
  }
}

export function managedRelayWebSocketUrl(relayHost: string): string {
  let url: URL;
  try {
    url = new URL(relayHost);
  } catch {
    throw new Error("Managed entitlement has an invalid relay origin");
  }
  if (
    url.protocol !== "https:" ||
    url.username !== "" ||
    url.password !== "" ||
    url.pathname !== "/" ||
    url.search !== "" ||
    url.hash !== "" ||
    !url.hostname
  ) {
    throw new Error("Managed entitlement relay must be an HTTPS origin");
  }
  url.protocol = "wss:";
  return url.toString().replace(/\/$/, "");
}

export function resolveManagedCommunityState(
  communities: Community[],
  entitlement: EvaosTeamsEntitlement,
  addedAt = new Date().toISOString(),
): ManagedCommunityState {
  const publicKey = entitlement.publicKey?.trim();
  if (!publicKey) {
    throw new Error("Managed entitlement is missing the verified public key");
  }
  const relayUrl = managedRelayWebSocketUrl(entitlement.relayHost);
  const existing = communities.find(
    (community) => community.id === entitlement.communityId,
  );
  const community: Community = {
    id: entitlement.communityId,
    name: existing?.name.trim() ? existing.name : "Hive",
    relayUrl,
    pubkey: publicKey,
    addedAt: existing?.addedAt ?? addedAt,
    ...(existing?.reposDir ? { reposDir: existing.reposDir } : {}),
  };
  const shouldRetire = (candidate: Community) =>
    (candidate.id === community.id &&
      (candidate.relayUrl !== community.relayUrl ||
        candidate.pubkey !== community.pubkey)) ||
    (candidate.relayUrl === community.relayUrl &&
      (candidate.id !== community.id || candidate.pubkey !== community.pubkey));
  const retiredCommunities = communities.filter((candidate) =>
    shouldRetire(candidate),
  );
  const survivingCommunities = communities.filter(
    (candidate) => !shouldRetire(candidate),
  );
  const existingIndex = survivingCommunities.findIndex(
    (candidate) => candidate.id === community.id,
  );
  const nextCommunities =
    existingIndex === -1
      ? [...survivingCommunities, community]
      : survivingCommunities.map((candidate, index) =>
          index === existingIndex ? community : candidate,
        );
  return {
    activeId: community.id,
    communities: nextCommunities,
    retiredCommunities,
  };
}

export function isManagedCommunityStateReady(
  communities: Community[],
  activeId: string | null,
  entitlement: EvaosTeamsEntitlement,
): boolean {
  if (activeId !== entitlement.communityId) {
    return false;
  }
  const publicKey = entitlement.publicKey?.trim();
  if (!publicKey) return false;
  const community = communities.find(
    (candidate) => candidate.id === entitlement.communityId,
  );
  if (!community) return false;
  return (
    community.relayUrl === managedRelayWebSocketUrl(entitlement.relayHost) &&
    community.pubkey === publicKey &&
    community.token === undefined
  );
}
