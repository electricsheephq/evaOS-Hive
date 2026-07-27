import { isManagedAgentActive } from "@/features/agents/lib/managedAgentControlActions";
import {
  isLocalWelcomeAgentRecord,
  isLocalWelcomePersonaId,
} from "@/features/onboarding/localWelcomeTeamPolicy";
import type { AgentPersona, ManagedAgent } from "@/shared/api/types";

type PersonaGroup = { persona: AgentPersona; agents: ManagedAgent[] };

export function buildUnifiedGroups(
  personas: AgentPersona[],
  agents: ManagedAgent[],
) {
  const byPersonaId = new Map<string, ManagedAgent[]>();
  const ungrouped: ManagedAgent[] = [];

  for (const agent of agents) {
    if (!agent.personaId) {
      ungrouped.push(agent);
    } else {
      const list = byPersonaId.get(agent.personaId) ?? [];
      list.push(agent);
      byPersonaId.set(agent.personaId, list);
    }
  }

  const matched = new Set<string>();
  const groups: PersonaGroup[] = personas.map((persona) => {
    matched.add(persona.id);
    return { persona, agents: byPersonaId.get(persona.id) ?? [] };
  });

  const unknown: ManagedAgent[] = [];
  for (const [id, list] of byPersonaId) {
    if (!matched.has(id)) unknown.push(...list);
  }

  return { groups, ungrouped, unknown };
}

export function filterManagedLocalWelcomeRecords(
  unifiedGroups: ReturnType<typeof buildUnifiedGroups>,
  managedProduct: boolean,
): ReturnType<typeof buildUnifiedGroups> {
  if (!managedProduct) {
    return unifiedGroups;
  }

  return {
    groups: unifiedGroups.groups
      .filter(({ persona }) => !isLocalWelcomePersonaId(persona.id))
      .map((group) => ({
        ...group,
        agents: group.agents.filter(
          (agent) => !isLocalWelcomeAgentRecord(agent),
        ),
      })),
    ungrouped: unifiedGroups.ungrouped.filter(
      (agent) => !isLocalWelcomeAgentRecord(agent),
    ),
    unknown: unifiedGroups.unknown.filter(
      (agent) => !isLocalWelcomeAgentRecord(agent),
    ),
  };
}

export function pickProfileAgent(agents: ManagedAgent[]) {
  return [...agents].sort((left, right) => {
    const activeDiff =
      Number(isManagedAgentActive(right)) - Number(isManagedAgentActive(left));
    if (activeDiff !== 0) return activeDiff;
    return left.name.localeCompare(right.name);
  })[0];
}
