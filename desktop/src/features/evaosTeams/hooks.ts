import { useQuery } from "@tanstack/react-query";

import { listHiveCompanyMembers } from "@/features/evaosTeams/api";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

export const hiveCompanyMembersQueryKey = ["hive-company-members"] as const;

export function useHiveCompanyMembersQuery(options?: { enabled?: boolean }) {
  const managed = desktopProductPolicy().managed;
  return useQuery({
    queryKey: hiveCompanyMembersQueryKey,
    queryFn: listHiveCompanyMembers,
    staleTime: 5_000,
    refetchInterval: 10_000,
    refetchIntervalInBackground: false,
    enabled: managed && (options?.enabled ?? true),
  });
}
