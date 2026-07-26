import type { HiveCompanyAgent } from "@/features/evaosTeams/api";
import type { RelayAgent } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export type CompanyAgentDirectoryEntry = RelayAgent & {
  /**
   * Distinguishes a real kind:10100 relay profile from an identity that exists
   * only in the authenticated company catalog. Relay invocation policy may be
   * inferred only from real relay profiles.
   */
  directorySource?: "relay" | "company-catalog";
};

export function mergeRelayAgentsWithCompanyCatalog(
  relayAgents: readonly RelayAgent[],
  companyAgents: readonly HiveCompanyAgent[],
): CompanyAgentDirectoryEntry[] {
  const merged = new Map<string, CompanyAgentDirectoryEntry>(
    relayAgents.map(
      (agent) =>
        [
          normalizePubkey(agent.pubkey),
          { ...agent, directorySource: "relay" as const },
        ] as const,
    ),
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
      directorySource: relayAgent ? "relay" : "company-catalog",
    });
  }

  return Array.from(merged.values()).sort((left, right) =>
    left.name.localeCompare(right.name),
  );
}

export function getRelayPolicyAgentPubkeys(
  agents: readonly CompanyAgentDirectoryEntry[] | undefined,
) {
  return new Set(
    (agents ?? [])
      .filter((agent) => agent.directorySource !== "company-catalog")
      .map((agent) => normalizePubkey(agent.pubkey)),
  );
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
