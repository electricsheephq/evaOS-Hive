import { isWelcomeExperienceChannel } from "@/features/onboarding/welcome";
import type { Channel } from "@/shared/api/types";

/** Hive keeps the native Buzz welcome-team lifecycle in every product mode. */
export function shouldUseLocalWelcomeTeam(_managedProduct: boolean): boolean {
  return true;
}

export function isLocalWelcomeExperienceChannel(
  channel: Channel | null | undefined,
): boolean {
  return isWelcomeExperienceChannel(channel);
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
  _managedProduct: boolean,
  _teamId: string,
): boolean {
  return true;
}
