import type { HiveCompanyAgent } from "@/features/evaosTeams/api";
import type {
  PresenceStatus,
  Profile,
  UserProfileSummary,
  UserSearchResult,
  UsersBatchResponse,
} from "@/shared/api/types";
import { normalizePubkey, truncatePubkey } from "@/shared/lib/pubkey";

type CompanyAgentProfile = Pick<
  Profile,
  | "about"
  | "avatarUrl"
  | "displayName"
  | "hasProfileEvent"
  | "nip05Handle"
  | "ownerPubkey"
  | "pubkey"
>;

function companyAgentByPubkey(
  agents: readonly HiveCompanyAgent[],
): ReadonlyMap<string, HiveCompanyAgent> {
  return new Map(
    agents.map((agent) => [normalizePubkey(agent.publicKey), agent] as const),
  );
}

export function isRawIdentityPlaceholder(
  value: string | null | undefined,
  pubkey: string,
): boolean {
  const trimmed = value?.trim().toLowerCase();
  const normalizedPubkey = normalizePubkey(pubkey);
  if (!trimmed) return true;
  if (trimmed === normalizedPubkey) return true;
  if (
    trimmed.replace("...", "…") ===
    truncatePubkey(normalizedPubkey).toLowerCase()
  ) {
    return true;
  }
  return (
    trimmed.startsWith("npub1") &&
    trimmed.length >= 50 &&
    !trimmed.includes("@")
  );
}

export function preferIdentityDisplayName(
  current: string | null | undefined,
  candidate: string | null | undefined,
  pubkey: string,
): string | null {
  const currentName = current?.trim() || null;
  if (currentName && !isRawIdentityPlaceholder(currentName, pubkey)) {
    return currentName;
  }
  const candidateName = candidate?.trim() || null;
  if (candidateName && !isRawIdentityPlaceholder(candidateName, pubkey)) {
    return candidateName;
  }
  return currentName ?? candidateName;
}

export function resolveCompanyAgentProfile(
  profile: CompanyAgentProfile,
  companyAgent: HiveCompanyAgent | undefined,
): Profile {
  if (!companyAgent) return profile;
  return {
    ...profile,
    displayName:
      preferIdentityDisplayName(
        profile.displayName,
        companyAgent.displayName,
        profile.pubkey,
      ) ?? companyAgent.displayName,
  };
}

function companyAgentSummary(
  pubkey: string,
  summary: UserProfileSummary | undefined,
  companyAgent: HiveCompanyAgent,
): UserProfileSummary {
  return {
    displayName:
      preferIdentityDisplayName(
        summary?.displayName,
        companyAgent.displayName,
        pubkey,
      ) ?? companyAgent.displayName,
    name: summary?.name ?? null,
    avatarUrl: summary?.avatarUrl ?? null,
    nip05Handle: summary?.nip05Handle ?? null,
    ownerPubkey: summary?.ownerPubkey ?? null,
    isAgent: true,
  };
}

export function resolveCompanyAgentBatch(
  response: UsersBatchResponse | undefined,
  requestedPubkeys: readonly string[],
  companyAgents: readonly HiveCompanyAgent[],
): UsersBatchResponse | undefined {
  if (!response) return undefined;
  const companyByPubkey = companyAgentByPubkey(companyAgents);
  const profiles = { ...response.profiles };
  const resolved = new Set<string>();

  for (const requestedPubkey of requestedPubkeys) {
    const pubkey = normalizePubkey(requestedPubkey);
    const companyAgent = companyByPubkey.get(pubkey);
    if (!companyAgent) continue;
    profiles[pubkey] = companyAgentSummary(
      pubkey,
      profiles[pubkey],
      companyAgent,
    );
    resolved.add(pubkey);
  }

  return {
    profiles,
    missing: response.missing.filter(
      (pubkey) => !resolved.has(normalizePubkey(pubkey)),
    ),
  };
}

export function mergeCompanyAgentSearchResults({
  companyAgents,
  limit,
  query,
  relayUsers,
}: {
  companyAgents: readonly HiveCompanyAgent[];
  limit: number;
  query: string;
  relayUsers: readonly UserSearchResult[];
}): UserSearchResult[] {
  const normalizedQuery = query.trim().toLowerCase();
  const byPubkey = new Map(
    relayUsers.map((user) => [normalizePubkey(user.pubkey), user] as const),
  );

  for (const companyAgent of companyAgents) {
    const pubkey = normalizePubkey(companyAgent.publicKey);
    const existing = byPubkey.get(pubkey);
    const displayName =
      preferIdentityDisplayName(
        existing?.displayName,
        companyAgent.displayName,
        pubkey,
      ) ?? companyAgent.displayName;
    if (
      normalizedQuery &&
      !displayName.toLowerCase().includes(normalizedQuery) &&
      !pubkey.includes(normalizedQuery)
    ) {
      continue;
    }
    byPubkey.set(pubkey, {
      pubkey,
      displayName,
      avatarUrl: existing?.avatarUrl ?? null,
      nip05Handle: existing?.nip05Handle ?? null,
      ownerPubkey: existing?.ownerPubkey ?? null,
      isAgent: true,
    });
  }

  return [...byPubkey.values()].slice(0, limit);
}

export type CompanyAgentPresence = "online" | "offline" | "unknown";

export function resolveCompanyAgentPresence(
  status: PresenceStatus | null | undefined,
): CompanyAgentPresence {
  if (status === "online") return "online";
  if (status === "offline") return "offline";
  return "unknown";
}
