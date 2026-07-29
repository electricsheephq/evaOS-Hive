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

const localWelcomePersonaIds = new Set<string>(LOCAL_WELCOME_PERSONA_IDS);

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
  managedProduct: boolean,
  persona: { id: string },
): boolean {
  return !managedProduct || !isLocalWelcomePersonaId(persona.id);
}

export function shouldPresentLocalWelcomeAgent(
  managedProduct: boolean,
  agent: {
    personaId?: string | null;
    teamId?: string | null;
  },
): boolean {
  return !managedProduct || !isLocalWelcomeAgentRecord(agent);
}

export function filterLocalWelcomePersonasForPresentation<
  T extends { id: string },
>(managedProduct: boolean, personas: readonly T[]): T[] {
  return personas.filter((persona) =>
    shouldPresentLocalWelcomePersona(managedProduct, persona),
  );
}

export function filterLocalWelcomeAgentsForPresentation<
  T extends {
    personaId?: string | null;
    teamId?: string | null;
  },
>(managedProduct: boolean, agents: readonly T[]): T[] {
  return agents.filter((agent) =>
    shouldPresentLocalWelcomeAgent(managedProduct, agent),
  );
}

export function shouldPresentLocalAgentTeam(
  managedProduct: boolean,
  teamId: string,
): boolean {
  return !managedProduct || teamId !== LOCAL_WELCOME_TEAM_ID;
}
