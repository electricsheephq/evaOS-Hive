import * as React from "react";

import { useRetireLocalWelcomePresentation } from "@/features/evaosTeams/hooks";
import type { AgentPersona, AgentTeam, ManagedAgent } from "@/shared/api/types";
import { normalizePubkey } from "@/shared/lib/pubkey";
import {
  filterLocalWelcomeAgentsForPresentation,
  filterLocalWelcomePersonasForPresentation,
  shouldPresentLocalAgentTeam,
  shouldPresentLocalWelcomeAgent,
} from "./localWelcomeTeamPolicy";

export function useLocalWelcomePresentation({
  enabled = true,
  managedAgents,
  personas = [],
  teams = [],
}: {
  enabled?: boolean;
  managedAgents: readonly ManagedAgent[];
  personas?: readonly AgentPersona[];
  teams?: readonly AgentTeam[];
}) {
  const retireLocalWelcomePresentation = useRetireLocalWelcomePresentation({
    enabled,
  });
  const visibleManagedAgents = React.useMemo(
    () =>
      filterLocalWelcomeAgentsForPresentation(
        retireLocalWelcomePresentation,
        managedAgents,
      ),
    [managedAgents, retireLocalWelcomePresentation],
  );
  const hiddenLocalWelcomeAgentPubkeys = React.useMemo(
    () =>
      new Set(
        managedAgents
          .filter(
            (agent) =>
              !shouldPresentLocalWelcomeAgent(
                retireLocalWelcomePresentation,
                agent,
              ),
          )
          .map((agent) => normalizePubkey(agent.pubkey)),
      ),
    [managedAgents, retireLocalWelcomePresentation],
  );
  const visiblePersonas = React.useMemo(
    () =>
      filterLocalWelcomePersonasForPresentation(
        retireLocalWelcomePresentation,
        personas,
      ),
    [personas, retireLocalWelcomePresentation],
  );
  const visibleTeams = React.useMemo(
    () =>
      teams.filter((team) =>
        shouldPresentLocalAgentTeam(retireLocalWelcomePresentation, team.id),
      ),
    [retireLocalWelcomePresentation, teams],
  );
  const personaNameByPubkey = React.useMemo(() => {
    const personaById = new Map(
      visiblePersonas.map((persona) => [persona.id, persona.displayName]),
    );
    return new Map(
      visibleManagedAgents.flatMap((agent) => {
        const displayName = agent.personaId
          ? personaById.get(agent.personaId)
          : undefined;
        return displayName
          ? [[normalizePubkey(agent.pubkey), displayName]]
          : [];
      }),
    );
  }, [visibleManagedAgents, visiblePersonas]);

  return {
    hiddenLocalWelcomeAgentPubkeys,
    personaNameByPubkey,
    retireLocalWelcomePresentation,
    visibleManagedAgents,
    visiblePersonas,
    visibleTeams,
  };
}
