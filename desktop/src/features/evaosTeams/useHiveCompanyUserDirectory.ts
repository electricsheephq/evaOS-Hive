import * as React from "react";
import { useQuery } from "@tanstack/react-query";

import { getEvaosTeamsAuthStatus } from "@/features/evaosTeams/api";
import { useHiveCompanyMembersQuery } from "@/features/evaosTeams/hooks";
import {
  companyDirectoryScope,
  companyMemberPubkeys,
  isManagedDirectoryCandidate,
  mergeCompanyDirectorySearchResults,
} from "@/features/evaosTeams/lib/companyMemberDirectory";
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
  const scopeQuery = useQuery({
    queryKey: ["hive-company-directory-scope"],
    queryFn: getEvaosTeamsAuthStatus,
    enabled: managed && enabled,
    gcTime: 0,
    staleTime: 0,
    refetchOnMount: "always",
  });
  const scope =
    !managed || scopeQuery.isFetching
      ? null
      : companyDirectoryScope(scopeQuery.data);
  const query = useHiveCompanyMembersQuery({ enabled, scope });
  const settled =
    !managed ||
    (!scopeQuery.isFetching && (scope === null || !query.isLoading));
  const members = React.useMemo(
    () =>
      settled && scope !== null && query.error === null
        ? (query.data ?? [])
        : [],
    [query.data, query.error, scope, settled],
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
  const memberPubkeys = React.useMemo(
    () => companyMemberPubkeys(members),
    [members],
  );
  const allows = React.useCallback(
    (candidate: Pick<UserSearchResult, "isAgent" | "pubkey">) =>
      isManagedDirectoryCandidate({ candidate, managed, memberPubkeys }),
    [managed, memberPubkeys],
  );
  return {
    allows,
    candidates,
    error:
      scopeQuery.error instanceof Error
        ? scopeQuery.error
        : query.error instanceof Error
          ? query.error
          : null,
    isLoading: !settled,
    managed,
    memberPubkeys,
    settled,
  };
}
