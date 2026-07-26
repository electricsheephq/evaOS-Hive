import { useQuery } from "@tanstack/react-query";

import { listHiveCompanyMembers } from "@/features/evaosTeams/api";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

export const hiveCompanyMembersQueryKey = (scope: string | null) =>
  ["hive-company-members", scope] as const;

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
