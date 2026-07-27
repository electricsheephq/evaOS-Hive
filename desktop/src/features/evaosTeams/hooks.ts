import { useQuery } from "@tanstack/react-query";

import {
  listHiveCompanyAgents,
  listHiveCompanyMembers,
} from "@/features/evaosTeams/api";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

export const hiveCompanyAgentsQueryKey = ["hive-company-agents"] as const;
export const hiveCompanyMembersQueryKey = (scope: string | null) =>
  ["hive-company-members", scope] as const;

export function useHiveCompanyAgentsQuery(options?: { enabled?: boolean }) {
  const managed = desktopProductPolicy().managed;
  return useQuery({
    queryKey: hiveCompanyAgentsQueryKey,
    queryFn: listHiveCompanyAgents,
    staleTime: 30_000,
    enabled: managed && (options?.enabled ?? true),
  });
}

export function useHiveCompanyMembersQuery(options: {
  enabled: boolean;
  scope: string | null;
}) {
  const managed = desktopProductPolicy().managed;
  return useQuery({
    queryKey: hiveCompanyMembersQueryKey(options.scope),
    queryFn: listHiveCompanyMembers,
    staleTime: 5_000,
    refetchInterval: 10_000,
    refetchIntervalInBackground: false,
    enabled: managed && options.enabled && options.scope !== null,
  });
}
