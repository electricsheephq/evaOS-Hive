import * as React from "react";
import { useQuery } from "@tanstack/react-query";

import { useRelayAgentsQuery } from "@/features/agents/hooks";
import { intersectAuthorizedCompanyAgents } from "@/features/agents/lib/companyAgentCatalog";
import { listHiveCompanyAgentAuthorizations } from "@/features/evaosTeams/api";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

export const companyAgentAuthorizationsQueryKey = [
  "hive-company-agent-authorizations",
] as const;

export function useCompanyVmAgents() {
  const managed = desktopProductPolicy().managed;
  const relayAgentsQuery = useRelayAgentsQuery({ enabled: managed });
  const authorizationsQuery = useQuery({
    enabled: managed,
    queryKey: companyAgentAuthorizationsQueryKey,
    queryFn: listHiveCompanyAgentAuthorizations,
    staleTime: 30_000,
    refetchInterval: 5 * 60_000,
    refetchIntervalInBackground: false,
  });
  const data = React.useMemo(
    () =>
      managed
        ? intersectAuthorizedCompanyAgents(
            relayAgentsQuery.data ?? [],
            authorizationsQuery.data ?? [],
          )
        : [],
    [authorizationsQuery.data, managed, relayAgentsQuery.data],
  );

  return {
    data,
    error: authorizationsQuery.error ?? relayAgentsQuery.error,
    isLoading:
      managed && (authorizationsQuery.isLoading || relayAgentsQuery.isLoading),
  };
}
