import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  getHiveCompanyAgentPolicy,
  listHiveCompanyAgents,
  listHiveCompanyMembers,
  setHiveCompanyAgentPolicy,
  type SetHiveCompanyAgentPolicyInput,
} from "@/features/evaosTeams/api";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

export const hiveCompanyAgentsQueryKey = ["hive-company-agents"] as const;
export const hiveCompanyMembersQueryKey = (scope: string | null) =>
  ["hive-company-members", scope] as const;
export const hiveCompanyAgentPolicyQueryKey = (agentInstanceId: string) =>
  ["hive-company-agent-policy", agentInstanceId] as const;

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

export function useHiveCompanyAgentPolicyQuery(options: {
  agentInstanceId: string | null;
  enabled: boolean;
}) {
  const managed = desktopProductPolicy().managed;
  const agentInstanceId = options.agentInstanceId;
  return useQuery({
    queryKey: hiveCompanyAgentPolicyQueryKey(agentInstanceId ?? "none"),
    queryFn: () => getHiveCompanyAgentPolicy(agentInstanceId ?? ""),
    staleTime: 2_000,
    refetchInterval: (query) =>
      query.state.data && query.state.data.status !== "applied" ? 5_000 : false,
    refetchIntervalInBackground: false,
    enabled: managed && options.enabled && agentInstanceId !== null,
  });
}

export function useSetHiveCompanyAgentPolicyMutation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (input: SetHiveCompanyAgentPolicyInput) =>
      setHiveCompanyAgentPolicy(input),
    onSuccess: (policy) => {
      queryClient.setQueryData(
        hiveCompanyAgentPolicyQueryKey(policy.agentInstanceId),
        policy,
      );
    },
    onError: (_error, input) => {
      void queryClient.invalidateQueries({
        queryKey: hiveCompanyAgentPolicyQueryKey(input.agentInstanceId),
      });
    },
  });
}
