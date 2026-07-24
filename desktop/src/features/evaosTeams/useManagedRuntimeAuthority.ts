import type { Community } from "@/features/communities/types";
import { useAutoRestartPolicy } from "@/features/agents/lib/useAutoRestartPolicy";
import { usePersonaSync } from "@/features/agents/lib/usePersonaSync";
import { useAgentObserverIngestion } from "@/features/agents/useAgentObserverIngestion";
import { useManagedAgentRuntimeReconciliation } from "@/features/agents/useManagedAgentRuntimeReconciliation";
import { useEvaosTeamsAuthority } from "@/features/evaosTeams/authority";

export function useManagedRuntimeAuthority(
  communities: Community[],
  identityPubkey?: string,
) {
  const { policy } = useEvaosTeamsAuthority();
  useManagedAgentRuntimeReconciliation(
    policy.canManageAgents ? communities : [],
  );
  usePersonaSync(policy.canManageAgents ? identityPubkey : undefined);
  useAutoRestartPolicy(policy.canManageAgents);
  useAgentObserverIngestion(policy.canBrowseAgents);
}
