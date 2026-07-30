import * as React from "react";
import { useQuery } from "@tanstack/react-query";

import { mergeCompanyAgentsWithRelay } from "@/features/agents/lib/companyAgentCatalog";
import { useHiveCompanyAgentsQuery } from "@/features/evaosTeams/hooks";
import { listRelayAgents } from "@/shared/api/tauri";

export const relayAgentsQueryKey = ["relay-agents"] as const;

/** Keep the native signed relay directory authoritative while attaching the
 * tenant-authorized company projection as a separate read-only view. */
export function useRelayAgentsQuery(options?: { enabled?: boolean }) {
  const relayAgentsQuery = useQuery({
    queryKey: relayAgentsQueryKey,
    queryFn: listRelayAgents,
    staleTime: 30_000,
    // Kind:10100 profiles have no `agents-data-changed` event path.
    refetchInterval: 5 * 60_000,
    refetchIntervalInBackground: false,
    enabled: options?.enabled,
  });
  const companyAgentsQuery = useHiveCompanyAgentsQuery(options);
  const companyVmAgents = React.useMemo(
    () =>
      mergeCompanyAgentsWithRelay(
        relayAgentsQuery.data ?? [],
        companyAgentsQuery.data ?? [],
      ),
    [companyAgentsQuery.data, relayAgentsQuery.data],
  );
  const companyAgentPubkeys = React.useMemo(
    () => new Set(companyVmAgents.map((agent) => agent.pubkey)),
    [companyVmAgents],
  );
  const companyAgentNamesByPubkey = React.useMemo(
    () => new Map(companyVmAgents.map((agent) => [agent.pubkey, agent.name])),
    [companyVmAgents],
  );

  return {
    ...relayAgentsQuery,
    companyAgentNamesByPubkey,
    companyAgentPubkeys,
    companyAgentsQuery,
    companyVmAgents,
  };
}
