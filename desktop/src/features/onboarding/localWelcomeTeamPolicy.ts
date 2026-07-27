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

export function shouldPresentLocalAgentTeam(
  managedProduct: boolean,
  teamId: string,
): boolean {
  return !managedProduct || teamId !== LOCAL_WELCOME_TEAM_ID;
}
