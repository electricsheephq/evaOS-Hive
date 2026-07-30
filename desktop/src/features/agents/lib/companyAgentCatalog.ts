import type {
  AgentPersona,
  AgentTeam,
  ManagedAgent,
  RelayAgent,
} from "@/shared/api/types";
import type { HiveCompanyAgentAuthorization } from "@/features/evaosTeams/api";
import { normalizePubkey } from "@/shared/lib/pubkey";

export const BUILTIN_WELCOME_TEAM_ID = "builtin-team:welcome";
export const BUILTIN_WELCOME_PERSONA_IDS = new Set([
  "builtin:fizz",
  "builtin:honey",
  "builtin:bumble",
]);
const CANONICAL_COMPANY_STARTER_IDS = new Set(["tars", "samantha", "hal-9000"]);

export type CompanyVmAgent = RelayAgent & {
  agentId: string | null;
  runtime: string;
};

function validPubkey(pubkey: string) {
  return /^[0-9a-f]{64}$/.test(normalizePubkey(pubkey));
}

/**
 * Intersect Electric's tenant-scoped authorization list with identities that
 * already exist in Buzz's native relay directory. Relay data remains the sole
 * source for name, rooms, routing, presence, profile, and capabilities.
 */
export function intersectAuthorizedCompanyAgents(
  relayAgents: RelayAgent[],
  authorizations: HiveCompanyAgentAuthorization[],
): CompanyVmAgent[] {
  const byPubkey = new Map<string, HiveCompanyAgentAuthorization>();
  for (const authorization of authorizations) {
    const publicKey = normalizePubkey(authorization.publicKey);
    const runtime = authorization.runtime.trim();
    if (!validPubkey(publicKey) || !runtime || byPubkey.has(publicKey)) {
      continue;
    }
    byPubkey.set(publicKey, authorization);
  }

  const seen = new Set<string>();
  return relayAgents.flatMap((agent) => {
    const publicKey = normalizePubkey(agent.pubkey);
    const authorization = byPubkey.get(publicKey);
    if (!authorization || seen.has(publicKey)) {
      return [];
    }
    seen.add(publicKey);
    return [
      {
        ...agent,
        agentId: authorization.agentId?.trim() || null,
        runtime: authorization.runtime.trim(),
      },
    ];
  });
}

export function resolveCompanyVmAgents(
  relayAgents: RelayAgent[],
  authorizations: HiveCompanyAgentAuthorization[],
  hasQueryError: boolean,
) {
  return hasQueryError
    ? []
    : intersectAuthorizedCompanyAgents(relayAgents, authorizations);
}

export function hasCanonicalCompanyWelcomeAgents(
  companyAgents: CompanyVmAgent[],
) {
  const canonicalIds = new Set(
    companyAgents
      .map((agent) => agent.agentId?.trim().toLowerCase())
      .filter((agentId): agentId is string => Boolean(agentId)),
  );
  return [...CANONICAL_COMPANY_STARTER_IDS].every((agentId) =>
    canonicalIds.has(agentId),
  );
}

export function filterBuiltinWelcomePersonas(
  personas: AgentPersona[],
  suppress: boolean,
) {
  return suppress
    ? personas.filter((persona) => !BUILTIN_WELCOME_PERSONA_IDS.has(persona.id))
    : personas;
}

export function filterBuiltinWelcomeAgents(
  agents: ManagedAgent[],
  suppress: boolean,
) {
  return suppress
    ? agents.filter(
        (agent) =>
          !(
            agent.personaId && BUILTIN_WELCOME_PERSONA_IDS.has(agent.personaId)
          ),
      )
    : agents;
}

export function filterBuiltinWelcomeTeams(
  teams: AgentTeam[],
  suppress: boolean,
) {
  return suppress
    ? teams.filter((team) => team.id !== BUILTIN_WELCOME_TEAM_ID)
    : teams;
}
