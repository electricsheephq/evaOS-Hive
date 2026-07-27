import type { HiveCompanyAgent } from "@/features/evaosTeams/api";
import { preferIdentityDisplayName } from "@/features/evaosTeams/lib/companyAgentIdentity";
import type { UserProfileLookup } from "@/features/profile/lib/identity";
import type { Channel, RelayAgent } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export function mergeRelayAgentsWithCompanyCatalog(
  relayAgents: readonly RelayAgent[],
  companyAgents: readonly HiveCompanyAgent[],
): RelayAgent[] {
  const merged = new Map(
    relayAgents.map((agent) => [normalizePubkey(agent.pubkey), agent] as const),
  );

  for (const companyAgent of companyAgents) {
    const pubkey = normalizePubkey(companyAgent.publicKey);
    const relayAgent = merged.get(pubkey);
    merged.set(pubkey, {
      pubkey,
      name:
        preferIdentityDisplayName(
          relayAgent?.name,
          companyAgent.displayName,
          pubkey,
        ) ?? companyAgent.displayName,
      agentType: companyAgent.runtime,
      channels: relayAgent?.channels ?? [],
      channelIds: relayAgent?.channelIds ?? [],
      capabilities: Array.from(
        new Set([
          ...(relayAgent?.capabilities ?? []),
          "company-vm",
          companyAgent.runtime,
        ]),
      ),
      status: relayAgent?.status ?? "unknown",
      respondTo: relayAgent?.respondTo ?? null,
      respondToAllowlist: relayAgent?.respondToAllowlist ?? [],
    });
  }

  return Array.from(merged.values()).sort((left, right) =>
    left.name.localeCompare(right.name),
  );
}

export function resolveCompanyAgentVisibleChannels(
  relayAgents: readonly RelayAgent[] | undefined,
  visibleChannels: readonly Channel[] | undefined,
): RelayAgent[] | undefined {
  if (!relayAgents) return undefined;
  if (!visibleChannels) return [...relayAgents];

  return relayAgents.map((agent) => {
    if (!agent.capabilities.includes("company-vm")) return agent;
    const pubkey = normalizePubkey(agent.pubkey);
    const memberChannels = visibleChannels.filter((channel) =>
      channel.memberPubkeys.some(
        (memberPubkey) => normalizePubkey(memberPubkey) === pubkey,
      ),
    );
    return {
      ...agent,
      channels: memberChannels.map((channel) => channel.name),
      channelIds: memberChannels.map((channel) => channel.id),
    };
  });
}

export function resolveRelayAgentProfiles(
  relayAgents: readonly RelayAgent[] | undefined,
  profiles: UserProfileLookup | undefined,
): RelayAgent[] | undefined {
  if (!relayAgents) return undefined;
  if (!profiles) return [...relayAgents];
  return relayAgents.map((agent) => {
    if (!agent.capabilities.includes("company-vm")) return agent;
    const profile = profiles[normalizePubkey(agent.pubkey)];
    const name = preferIdentityDisplayName(
      profile?.displayName,
      agent.name,
      agent.pubkey,
    );
    return name && name !== agent.name ? { ...agent, name } : agent;
  });
}

export function companyVmAgentsFromCatalog(
  relayAgents: readonly RelayAgent[],
  companyAgents: readonly HiveCompanyAgent[],
): RelayAgent[] {
  const catalogPubkeys = new Set(
    companyAgents.map((agent) => normalizePubkey(agent.publicKey)),
  );
  return mergeRelayAgentsWithCompanyCatalog(relayAgents, companyAgents).filter(
    (agent) => catalogPubkeys.has(normalizePubkey(agent.pubkey)),
  );
}
