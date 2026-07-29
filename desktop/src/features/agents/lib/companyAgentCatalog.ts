import type { HiveCompanyAgent } from "@/features/evaosTeams/api";
import type { ManagedAgent, RelayAgent } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";

export type CompanyVmAgent = Omit<RelayAgent, "status"> & {
  agentInstanceId: string;
  agentId?: string;
  runtime: string;
  status: RelayAgent["status"] | "unknown";
};

const LOCAL_WELCOME_TEAM_ID = "builtin-team:welcome";
const LOCAL_WELCOME_PERSONA_IDS = new Set([
  "builtin:fizz",
  "builtin:honey",
  "builtin:bumble",
]);
const CANONICAL_COMPANY_WELCOME_AGENT_IDS = [
  "tars",
  "samantha",
  "hal-9000",
] as const;

/**
 * Project the tenant-authorized catalog into the native relay-agent shape.
 * Signed relay records remain authoritative for collaboration fields. Catalog
 * runtime and IDs are presentation/classification metadata only.
 */
export function mergeCompanyAgentsWithRelay(
  relayAgents: readonly RelayAgent[],
  companyAgents: readonly HiveCompanyAgent[],
): CompanyVmAgent[] {
  const relayByPubkey = new Map(
    relayAgents.map((agent) => [normalizePubkey(agent.pubkey), agent] as const),
  );
  const merged = new Map<string, CompanyVmAgent>();

  for (const companyAgent of companyAgents) {
    const pubkey = normalizePubkey(companyAgent.publicKey);
    if (merged.has(pubkey)) continue;
    const relayAgent = relayByPubkey.get(pubkey);
    merged.set(pubkey, {
      agentInstanceId: companyAgent.agentInstanceId,
      ...(companyAgent.agentId ? { agentId: companyAgent.agentId } : {}),
      pubkey,
      name: relayAgent?.name?.trim() || companyAgent.displayName,
      agentType: relayAgent?.agentType?.trim() || companyAgent.runtime,
      runtime: companyAgent.runtime,
      channels: relayAgent?.channels ?? [],
      channelIds: relayAgent?.channelIds ?? [],
      // Catalog runtime labels never become invocation capabilities.
      capabilities: relayAgent?.capabilities ?? [],
      status: relayAgent?.status ?? "unknown",
      respondTo: relayAgent?.respondTo ?? null,
      respondToAllowlist: relayAgent?.respondToAllowlist ?? [],
    });
  }

  return [...merged.values()].sort((left, right) =>
    left.name.localeCompare(right.name),
  );
}

export function excludeLocalPubkeyDuplicates<T extends { pubkey: string }>(
  companyAgents: readonly T[],
  localAgents: readonly Pick<ManagedAgent, "pubkey">[],
): T[] {
  const localPubkeys = new Set(
    localAgents.map((agent) => normalizePubkey(agent.pubkey)),
  );
  return companyAgents.filter(
    (agent) => !localPubkeys.has(normalizePubkey(agent.pubkey)),
  );
}

export function hasCanonicalCompanyWelcomeAgents(
  agents: ReadonlyArray<Pick<HiveCompanyAgent, "agentId" | "runtime">>,
): boolean {
  const canonicalIds = new Set(
    agents
      .filter((agent) => agent.runtime.trim().toLowerCase() === "hermes")
      .map((agent) => agent.agentId?.trim().toLowerCase())
      .filter((agentId): agentId is string => Boolean(agentId)),
  );
  return CANONICAL_COMPANY_WELCOME_AGENT_IDS.every((agentId) =>
    canonicalIds.has(agentId),
  );
}

function isLocalWelcomeAgent(agent: {
  personaId?: string | null;
  teamId?: string | null;
}) {
  return (
    agent.teamId === LOCAL_WELCOME_TEAM_ID ||
    (agent.personaId !== null &&
      agent.personaId !== undefined &&
      LOCAL_WELCOME_PERSONA_IDS.has(agent.personaId))
  );
}

export function filterLocalWelcomeAgents<
  T extends {
    personaId?: string | null;
    teamId?: string | null;
  },
>(suppress: boolean, agents: readonly T[]): T[] {
  return suppress
    ? agents.filter((agent) => !isLocalWelcomeAgent(agent))
    : [...agents];
}

export function filterLocalWelcomePersonas<T extends { id: string }>(
  suppress: boolean,
  personas: readonly T[],
): T[] {
  return suppress
    ? personas.filter((persona) => !LOCAL_WELCOME_PERSONA_IDS.has(persona.id))
    : [...personas];
}

export function filterLocalWelcomeTeams<T extends { id: string }>(
  suppress: boolean,
  teams: readonly T[],
): T[] {
  return suppress
    ? teams.filter((team) => team.id !== LOCAL_WELCOME_TEAM_ID)
    : [...teams];
}
