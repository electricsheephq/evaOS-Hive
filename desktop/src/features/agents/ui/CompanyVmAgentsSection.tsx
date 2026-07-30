import { useUserProfileQuery } from "@/features/profile/hooks";
import type { CompanyVmAgent } from "@/features/agents/lib/companyAgentCatalog";
import type { ProfilePanelOpenOptions } from "@/shared/context/ProfilePanelContext";
import { Badge } from "@/shared/ui/badge";
import { IdentityCardSkeleton } from "@/shared/ui/identity-card-skeleton";
import { SectionHeader } from "@/shared/ui/PageHeader";
import { AgentIdentityCard } from "./AgentIdentityCard";

const GRID_CLASS =
  "grid w-full grid-cols-[repeat(auto-fill,minmax(220px,240px))] justify-start gap-3";

export function CompanyVmAgentsSection({
  agents,
  error,
  isLoading,
  onOpenProfile,
}: {
  agents: CompanyVmAgent[];
  error: Error | null;
  isLoading: boolean;
  onOpenProfile: (pubkey: string, options?: ProfilePanelOpenOptions) => void;
}) {
  if (!isLoading && !error && agents.length === 0) {
    return null;
  }

  return (
    <section className="space-y-4" data-testid="company-vm-agents">
      <SectionHeader
        title="Company VM agents"
        description="Native Hive identities hosted by your company Hermes gateway."
      />
      {isLoading ? (
        <div className={GRID_CLASS}>
          <IdentityCardSkeleton
            footerSubtitleWidthClass="w-20"
            footerTitleWidthClass="w-24"
          />
        </div>
      ) : null}
      {!isLoading && agents.length > 0 ? (
        <div className={GRID_CLASS}>
          {agents.map((agent) => (
            <CompanyVmAgentCard
              agent={agent}
              key={agent.pubkey}
              onOpenProfile={onOpenProfile}
            />
          ))}
        </div>
      ) : null}
      {error ? (
        <p className="rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error.message}
        </p>
      ) : null}
    </section>
  );
}

function CompanyVmAgentCard({
  agent,
  onOpenProfile,
}: {
  agent: CompanyVmAgent;
  onOpenProfile: (pubkey: string, options?: ProfilePanelOpenOptions) => void;
}) {
  const profileQuery = useUserProfileQuery(agent.pubkey);
  const label = profileQuery.data?.displayName?.trim() || agent.name;

  return (
    <AgentIdentityCard
      ariaLabel={`${label} company VM agent profile`}
      avatarUrl={profileQuery.data?.avatarUrl}
      dataTestId={`company-vm-agent-${agent.pubkey}`}
      label={label}
      modelLabel="Company VM"
      onClick={() => onOpenProfile(agent.pubkey)}
      statusBadge={
        <Badge className="mt-1 w-fit" variant="secondary">
          {agent.status}
        </Badge>
      }
    />
  );
}
