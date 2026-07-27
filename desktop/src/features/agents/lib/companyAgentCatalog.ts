import type { HiveCompanyAgent } from "@/features/evaosTeams/api";
import {
  preferIdentityDisplayName,
  resolveCompanyAgentPresence,
} from "@/features/evaosTeams/lib/companyAgentIdentity";
import type { UserProfileLookup } from "@/features/profile/lib/identity";
import type { Channel, PresenceLookup, RelayAgent } from "@/shared/api/types";
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
  companyAgentPubkeys: ReadonlySet<string>,
): RelayAgent[] | undefined {
  if (!relayAgents) return undefined;

  return relayAgents.map((agent) => {
    const pubkey = normalizePubkey(agent.pubkey);
    if (!companyAgentPubkeys.has(pubkey)) return agent;
    if (!visibleChannels) {
      return agent.channels.length === 0 && agent.channelIds.length === 0
        ? agent
        : { ...agent, channels: [], channelIds: [] };
    }
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
  companyAgentPubkeys: ReadonlySet<string>,
): RelayAgent[] | undefined {
  if (!relayAgents) return undefined;
  if (!profiles) return [...relayAgents];
  return relayAgents.map((agent) => {
    const pubkey = normalizePubkey(agent.pubkey);
    if (!companyAgentPubkeys.has(pubkey)) return agent;
    const profile = profiles[pubkey];
    const name = preferIdentityDisplayName(
      profile?.displayName,
      agent.name,
      agent.pubkey,
    );
    return name && name !== agent.name ? { ...agent, name } : agent;
  });
}

export function resolveCompanyAgentPresenceStatuses(
  relayAgents: readonly RelayAgent[] | undefined,
  presence: PresenceLookup | undefined,
  companyAgentPubkeys: ReadonlySet<string>,
): RelayAgent[] | undefined {
  if (!relayAgents) return undefined;
  return relayAgents.map((agent) => {
    const pubkey = normalizePubkey(agent.pubkey);
    if (!companyAgentPubkeys.has(pubkey)) return agent;
    const status = resolveCompanyAgentPresence(presence?.[pubkey]);
    return status === agent.status ? agent : { ...agent, status };
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
