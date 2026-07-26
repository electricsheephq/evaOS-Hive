import * as React from "react";

import { useHiveCompanyMembersQuery } from "@/features/evaosTeams/hooks";
import { mergeCompanyDirectorySearchResults } from "@/features/evaosTeams/lib/companyMemberDirectory";
import type { UserSearchResult } from "@/shared/api/types";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

export function useHiveCompanyUserDirectory({
  enabled,
  relayUsers,
}: {
  enabled: boolean;
  relayUsers: readonly UserSearchResult[];
}) {
  const managed = desktopProductPolicy().managed;
  const query = useHiveCompanyMembersQuery({ enabled });
  const members = React.useMemo(
    () => (query.error === null ? (query.data ?? []) : []),
    [query.data, query.error],
  );
  const candidates = React.useMemo(
    () =>
      mergeCompanyDirectorySearchResults({
        managed,
        members,
        relayUsers,
      }),
    [managed, members, relayUsers],
  );
  return {
    candidates,
    error: query.error instanceof Error ? query.error : null,
    isLoading: query.isLoading,
  };
}
