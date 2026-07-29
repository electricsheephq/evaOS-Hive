import * as React from "react";
import { useQuery } from "@tanstack/react-query";

import {
  mergeRelayAgentsWithCompanyCatalog,
  resolveCompanyAgentPresenceStatuses,
  resolveCompanyAgentVisibleChannels,
  resolveRelayAgentProfiles,
} from "@/features/agents/lib/companyAgentCatalog";
import { useChannelsQuery } from "@/features/channels/hooks";
import { useHiveCompanyAgentsQuery } from "@/features/evaosTeams/hooks";
import { usePresenceQuery } from "@/features/presence/hooks";
import { useUsersBatchQuery } from "@/features/profile/hooks";
import { listRelayAgents } from "@/shared/api/tauri";
import { normalizePubkey } from "@/shared/lib/pubkey";

export const relayAgentsQueryKey = ["relay-agents"] as const;

export function useRelayAgentsQuery(options?: { enabled?: boolean }) {
  const relayAgentsQuery = useQuery({
    queryKey: relayAgentsQueryKey,
    queryFn: listRelayAgents,
    staleTime: 30_000,
    // Relay agent profiles (kind:10100) are near-static and the backing
    // `list_relay_agents` command is an unfiltered relay query for the whole
    // profile set — mounted on ~13 always-live surfaces (channel screen,
    // members bar, mentions, sidebar, profile popovers), so a tight interval
    // re-pulls the full set app-wide. This poll is also the ONLY refresh path:
    // the `agents-data-changed` event fires only for local persona/team/managed
    // reconcile (kinds PERSONA/TEAM/MANAGED_AGENT), never for kind:10100. So we
    // keep polling but at a relaxed cadence and pause it while backgrounded.
    refetchInterval: 5 * 60_000,
    refetchIntervalInBackground: false,
    enabled: options?.enabled,
  });
  const companyAgentsQuery = useHiveCompanyAgentsQuery(options);
  const companyAgentPubkeys = React.useMemo(
    () =>
      new Set(
        (companyAgentsQuery.data ?? []).map((agent) =>
          normalizePubkey(agent.publicKey),
        ),
      ),
    [companyAgentsQuery.data],
  );
  const mergedRelayAgents = React.useMemo(
    () =>
      (companyAgentsQuery.data?.length ?? 0) > 0
        ? mergeRelayAgentsWithCompanyCatalog(
            relayAgentsQuery.data ?? [],
            companyAgentsQuery.data ?? [],
          )
        : (relayAgentsQuery.data ?? []),
    [companyAgentsQuery.data, relayAgentsQuery.data],
  );
  const hasCompanyVmAgents = companyAgentPubkeys.size > 0;
  const channelsQuery = useChannelsQuery({
    enabled: (options?.enabled ?? true) && hasCompanyVmAgents,
  });
  const agentPubkeys = React.useMemo(
    () => [...companyAgentPubkeys],
    [companyAgentPubkeys],
  );
  const profilesQuery = useUsersBatchQuery(agentPubkeys, {
    enabled: (options?.enabled ?? true) && agentPubkeys.length > 0,
  });
  const presenceQuery = usePresenceQuery(agentPubkeys, {
    enabled: (options?.enabled ?? true) && agentPubkeys.length > 0,
  });
  const sourceAgents =
    relayAgentsQuery.data === undefined && companyAgentsQuery.data === undefined
      ? undefined
      : mergedRelayAgents;
  const agentsWithProfiles = React.useMemo(
    () =>
      hasCompanyVmAgents
        ? resolveRelayAgentProfiles(
            sourceAgents,
            profilesQuery.data?.profiles,
            companyAgentPubkeys,
          )
        : sourceAgents,
    [
      companyAgentPubkeys,
      hasCompanyVmAgents,
      profilesQuery.data?.profiles,
      sourceAgents,
    ],
  );
  const agentsWithChannels = React.useMemo(
    () =>
      hasCompanyVmAgents
        ? resolveCompanyAgentVisibleChannels(
            agentsWithProfiles,
            channelsQuery.data,
            companyAgentPubkeys,
          )
        : agentsWithProfiles,
    [
      agentsWithProfiles,
      channelsQuery.data,
      companyAgentPubkeys,
      hasCompanyVmAgents,
    ],
  );
  const resolvedAgents = React.useMemo(
    () =>
      hasCompanyVmAgents
        ? resolveCompanyAgentPresenceStatuses(
            agentsWithChannels,
            presenceQuery.data,
            companyAgentPubkeys,
          )
        : agentsWithChannels,
    [
      agentsWithChannels,
      companyAgentPubkeys,
      hasCompanyVmAgents,
      presenceQuery.data,
    ],
  );

  return {
    ...relayAgentsQuery,
    companyAgentPubkeys,
    data: resolvedAgents,
  };
}
