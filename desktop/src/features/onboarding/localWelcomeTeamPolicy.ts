import { isWelcomeExperienceChannel } from "@/features/onboarding/welcome";
import type { Channel } from "@/shared/api/types";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

/**
 * Managed Hive communities use company-registered VM agents. The native local
 * welcome team remains available only to unmanaged Buzz.
 */
export function shouldUseLocalWelcomeTeam(managedProduct: boolean): boolean {
  return !managedProduct;
}

export function isLocalWelcomeExperienceChannel(
  channel: Channel | null | undefined,
): boolean {
  return (
    shouldUseLocalWelcomeTeam(desktopProductPolicy().managed) &&
    isWelcomeExperienceChannel(channel)
  );
}

export const LOCAL_WELCOME_TEAM_ID = "builtin-team:welcome";
export const LOCAL_WELCOME_PERSONA_IDS = [
  "builtin:fizz",
  "builtin:honey",
  "builtin:bumble",
] as const;
export const CANONICAL_COMPANY_WELCOME_AGENT_IDS = [
  "tars",
  "samantha",
  "hal-9000",
] as const;

const localWelcomePersonaIds = new Set<string>(LOCAL_WELCOME_PERSONA_IDS);
const canonicalCompanyWelcomeAgentIds = new Set<string>(
  CANONICAL_COMPANY_WELCOME_AGENT_IDS,
);

export function hasCanonicalCompanyWelcomeAgents(
  agents: ReadonlyArray<{
    agentId?: string | null;
    runtime: string;
  }>,
): boolean {
  const availableIds = new Set(
    agents
      .filter((agent) => agent.runtime.trim().toLowerCase() === "hermes")
      .map((agent) => agent.agentId?.trim().toLowerCase())
      .filter((agentId): agentId is string => agentId !== undefined)
      .filter((agentId) => canonicalCompanyWelcomeAgentIds.has(agentId)),
  );

  return CANONICAL_COMPANY_WELCOME_AGENT_IDS.every((agentId) =>
    availableIds.has(agentId),
  );
}

export function isLocalWelcomePersonaId(
  personaId: string | null | undefined,
): boolean {
  return (
    personaId !== null &&
    personaId !== undefined &&
    localWelcomePersonaIds.has(personaId)
  );
}

export function isLocalWelcomeAgentRecord(agent: {
  personaId?: string | null;
  teamId?: string | null;
}): boolean {
  return (
    agent.teamId === LOCAL_WELCOME_TEAM_ID ||
    isLocalWelcomePersonaId(agent.personaId)
  );
}

export function shouldRunLocalWelcomeAgent(
  managedProduct: boolean,
  agent: {
    personaId?: string | null;
    teamId?: string | null;
  },
): boolean {
  return !managedProduct || !isLocalWelcomeAgentRecord(agent);
}

export function shouldPresentLocalWelcomePersona(
  retireLocalWelcomePresentation: boolean,
  persona: { id: string },
): boolean {
  return (
    !retireLocalWelcomePresentation || !isLocalWelcomePersonaId(persona.id)
  );
}

export function shouldPresentLocalWelcomeAgent(
  retireLocalWelcomePresentation: boolean,
  agent: {
    personaId?: string | null;
    teamId?: string | null;
  },
): boolean {
  return !retireLocalWelcomePresentation || !isLocalWelcomeAgentRecord(agent);
}

export function filterLocalWelcomePersonasForPresentation<
  T extends { id: string },
>(retireLocalWelcomePresentation: boolean, personas: readonly T[]): T[] {
  return personas.filter((persona) =>
    shouldPresentLocalWelcomePersona(retireLocalWelcomePresentation, persona),
  );
}

export function filterLocalWelcomeAgentsForPresentation<
  T extends {
    personaId?: string | null;
    teamId?: string | null;
  },
>(retireLocalWelcomePresentation: boolean, agents: readonly T[]): T[] {
  return agents.filter((agent) =>
    shouldPresentLocalWelcomeAgent(retireLocalWelcomePresentation, agent),
  );
}

export function shouldPresentLocalAgentTeam(
  retireLocalWelcomePresentation: boolean,
  teamId: string,
): boolean {
  return !retireLocalWelcomePresentation || teamId !== LOCAL_WELCOME_TEAM_ID;
}
