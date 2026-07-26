import type { HiveCompanyAgent } from "@/features/evaosTeams/api";
import type { RelayAgent } from "@/shared/api/types";
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
      name: companyAgent.displayName,
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
      status: relayAgent?.status ?? "offline",
      respondTo: relayAgent?.respondTo ?? null,
      respondToAllowlist: relayAgent?.respondToAllowlist ?? [],
    });
  }

  return Array.from(merged.values()).sort((left, right) =>
    left.name.localeCompare(right.name),
  );
}

export function filterCompanyVmAgents(
  agents: readonly RelayAgent[],
): RelayAgent[] {
  return agents.filter((agent) => agent.capabilities.includes("company-vm"));
}
