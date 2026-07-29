import { useQuery } from "@tanstack/react-query";

import { listHiveCompanyAgents } from "./api";

export const hiveCompanyAgentsQueryKey = ["hive-company-agents"] as const;

export function useHiveCompanyAgentsQuery(options?: { enabled?: boolean }) {
  return useQuery({
    enabled: options?.enabled ?? true,
    queryKey: hiveCompanyAgentsQueryKey,
    queryFn: listHiveCompanyAgents,
    staleTime: 30_000,
  });
}
