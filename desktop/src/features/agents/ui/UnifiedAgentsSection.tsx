import * as React from "react";
import { ChevronDown, ChevronRight, RefreshCw } from "lucide-react";

import { formatAgentModelLabel } from "@/features/agents/lib/formatAgentModelLabel";
import { friendlyAgentLastError } from "@/features/agents/lib/friendlyAgentLastError";
import { isManagedAgentActive } from "@/features/agents/lib/managedAgentControlActions";
import { useUserProfileQuery } from "@/features/profile/hooks";
import type {
  AgentPersona,
  ManagedAgent,
  RelayAgent,
} from "@/shared/api/types";
import type { ProfilePanelOpenOptions } from "@/shared/context/ProfilePanelContext";
import { useFeedbackToasts } from "@/shared/hooks/useToastEffect";
import { useFileImportZone } from "@/shared/hooks/useFileImportZone";
import { Badge } from "@/shared/ui/badge";
import { desktopProductPolicy } from "@/shared/product/productIdentity";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/shared/ui/dropdown-menu";
import { IdentityCardSkeleton } from "@/shared/ui/identity-card-skeleton";
import { AgentIdentityCard } from "./AgentIdentityCard";
import { AgentRuntimeAvatarControl } from "./AgentRuntimeAvatarControl";
import { CreateIdentityCard } from "./CreateIdentityCard";
import { PersonaActionsMenu } from "./PersonaActionsMenu";
import {
  buildUnifiedGroups,
  filterManagedLocalWelcomeRecords,
  pickProfileAgent,
} from "./unifiedAgentGroups";

type UnifiedAgentsSectionProps = {
  defaultModel: string;
  actionErrorMessage: string | null;
  actionNoticeMessage: string | null;
  agents: ManagedAgent[];
  agentsError: Error | null;
  isActionPending: boolean;
  isAgentsLoading: boolean;
  startingAgentPubkey: string | null;
  startingPersonaIds: ReadonlySet<string>;
  onOpenAgentProfile: (
    pubkey: string,
    options?: ProfilePanelOpenOptions,
  ) => void;
  onOpenPersonaProfile: (persona: AgentPersona) => void;
  onStartAgent: (pubkey: string) => void;
  onStartPersona: (persona: AgentPersona) => void;
  canChooseCatalog: boolean;
  personas: AgentPersona[];
  personasError: Error | null;
  personaFeedbackErrorMessage: string | null;
  personaFeedbackNoticeMessage: string | null;
  isPersonasLoading: boolean;
  isPersonasPending: boolean;
  onCreatePersona: () => void;
  onChooseCatalog: () => void;
  onDuplicatePersona: (persona: AgentPersona) => void;
  onEditPersona: (persona: AgentPersona) => void;
  onSharePersona: (
    persona: AgentPersona,
    linkedAgent: ManagedAgent | undefined,
  ) => void;
  onDeactivatePersona: (persona: AgentPersona) => void;
  onDeletePersona: (persona: AgentPersona) => void;
  onImportSnapshotFile: (fileBytes: number[], fileName: string) => void;
  companyAgents: RelayAgent[];
  companyAgentsError: Error | null;
  isCompanyAgentsLoading: boolean;
  onRefreshCompanyAgents: () => void;
};

const AGENT_CARD_COLUMN_CLASS = "w-full";
const AGENT_CARD_GRID_CLASS = `${AGENT_CARD_COLUMN_CLASS} mx-auto grid max-w-[996px] grid-cols-[repeat(auto-fill,minmax(220px,240px))] justify-center gap-3`;

export function UnifiedAgentsSection(props: UnifiedAgentsSectionProps) {
  const {
    actionErrorMessage,
    actionNoticeMessage,
    defaultModel,
    agents,
    agentsError,
    isActionPending,
    isAgentsLoading,
    startingAgentPubkey,
    startingPersonaIds,
    onOpenAgentProfile,
    onOpenPersonaProfile,
    onStartAgent,
    onStartPersona,
    canChooseCatalog,
    personas,
    personasError,
    personaFeedbackErrorMessage,
    personaFeedbackNoticeMessage,
    isPersonasLoading,
    isPersonasPending,
    onCreatePersona,
    onChooseCatalog,
    onDuplicatePersona,
    onEditPersona,
    onSharePersona,
    onDeactivatePersona,
    onDeletePersona,
    onImportSnapshotFile,
    companyAgents,
    companyAgentsError,
    isCompanyAgentsLoading,
    onRefreshCompanyAgents,
  } = props;
  const isManagedHive = desktopProductPolicy().managed;

  const { groups, ungrouped, unknown } = React.useMemo(
    () =>
      filterManagedLocalWelcomeRecords(
        buildUnifiedGroups(personas, agents),
        isManagedHive,
      ),
    [agents, isManagedHive, personas],
  );
  const [collapsed, setCollapsed] = React.useState<Set<string>>(new Set());
  const {
    fileInputRef,
    isDragOver,
    dropHandlers,
    handleFileChange,
    openFilePicker,
  } = useFileImportZone({ onImportFile: onImportSnapshotFile });

  function toggle(key: string) {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }

  useFeedbackToasts(actionNoticeMessage, actionErrorMessage);
  useFeedbackToasts(personaFeedbackNoticeMessage, personaFeedbackErrorMessage);
  const isLoading = isAgentsLoading || isPersonasLoading;

  return (
    <section
      className="relative space-y-4"
      data-testid="agents-library-personas"
      {...dropHandlers}
    >
      {isDragOver ? (
        <div className="pointer-events-none absolute -inset-1 z-10 flex items-center justify-center rounded-2xl border-2 border-dashed border-primary/50 bg-background/80 backdrop-blur-sm">
          <p className="text-sm font-medium text-primary">
            Drop .agent.json or .agent.png to import
          </p>
        </div>
      ) : null}

      <input
        accept=".agent.json,.agent.png"
        className="hidden"
        onChange={handleFileChange}
        ref={fileInputRef}
        type="file"
      />

      {isLoading ? <LoadingSkeleton /> : null}

      {!isLoading ? (
        <div className="space-y-3" data-testid="unified-agents-groups">
          <div className={AGENT_CARD_GRID_CLASS}>
            {groups.map((group) => {
              const profileAgent = pickProfileAgent(group.agents);
              return (
                <AgentPersonaCard
                  actions={
                    <PersonaActionsMenu
                      isActionPending={isActionPending}
                      isPending={isPersonasPending}
                      persona={group.persona}
                      linkedAgent={profileAgent}
                      onDeactivate={onDeactivatePersona}
                      onDelete={onDeletePersona}
                      onDuplicate={onDuplicatePersona}
                      onEdit={onEditPersona}
                      onShare={onSharePersona}
                    />
                  }
                  agent={profileAgent}
                  defaultModel={defaultModel}
                  key={group.persona.id}
                  persona={group.persona}
                  startingAgentPubkey={startingAgentPubkey}
                  startingPersonaIds={startingPersonaIds}
                  onOpenAgentProfile={onOpenAgentProfile}
                  onOpenPersonaProfile={onOpenPersonaProfile}
                  onStartAgent={onStartAgent}
                  onStartPersona={onStartPersona}
                />
              );
            })}
            <NewAgentCard
              canChooseCatalog={canChooseCatalog}
              isManagedHive={isManagedHive}
              isPersonasPending={isPersonasPending}
              openFilePicker={openFilePicker}
              onChooseCatalog={onChooseCatalog}
              onCreatePersona={onCreatePersona}
              onRefreshCompanyAgents={onRefreshCompanyAgents}
            />
          </div>

          {isManagedHive ? (
            <CompanyVmAgentsSection
              agents={companyAgents}
              error={companyAgentsError}
              isLoading={isCompanyAgentsLoading}
              onOpenAgentProfile={onOpenAgentProfile}
            />
          ) : null}

          {unknown.length > 0 ? (
            <CollapsibleAgentGroup
              agents={unknown}
              collapsed={collapsed}
              defaultModel={defaultModel}
              groupKey="__unknown__"
              label="Unknown agents"
              startingAgentPubkey={startingAgentPubkey}
              onToggle={toggle}
              onOpenAgentProfile={onOpenAgentProfile}
              onStartAgent={onStartAgent}
            />
          ) : null}
          {ungrouped.length > 0 ? (
            <CollapsibleAgentGroup
              agents={ungrouped}
              collapsed={collapsed}
              defaultModel={defaultModel}
              groupKey="__ungrouped__"
              label="Custom agents"
              startingAgentPubkey={startingAgentPubkey}
              onToggle={toggle}
              onOpenAgentProfile={onOpenAgentProfile}
              onStartAgent={onStartAgent}
            />
          ) : null}
        </div>
      ) : null}

      {agentsError ? (
        <p
          className={`${AGENT_CARD_COLUMN_CLASS} rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive`}
        >
          {agentsError.message}
        </p>
      ) : null}
      {personasError ? (
        <p
          className={`${AGENT_CARD_COLUMN_CLASS} rounded-2xl border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive`}
        >
          {personasError.message}
        </p>
      ) : null}
    </section>
  );
}

function AgentPersonaCard({
  actions,
  agent,
  defaultModel,
  persona,
  startingAgentPubkey,
  startingPersonaIds,
  onOpenAgentProfile,
  onOpenPersonaProfile,
  onStartAgent,
  onStartPersona,
}: {
  actions?: React.ReactNode;
  agent: ManagedAgent | undefined;
  defaultModel: string;
  persona: AgentPersona;
  startingAgentPubkey: string | null;
  startingPersonaIds: ReadonlySet<string>;
  onOpenAgentProfile: (
    pubkey: string,
    options?: ProfilePanelOpenOptions,
  ) => void;
  onOpenPersonaProfile: (persona: AgentPersona) => void;
  onStartAgent: (pubkey: string) => void;
  onStartPersona: (persona: AgentPersona) => void;
}) {
  const title = persona.displayName;
  const explicitModel = agent?.model ?? persona.model;
  const modelLabel = explicitModel?.trim()
    ? formatAgentModelLabel(explicitModel)
    : formatDefaultModelLabel(defaultModel);
  const isActive = agent ? isManagedAgentActive(agent) : false;
  const profileQuery = useUserProfileQuery(agent?.pubkey);
  const avatarUrl = agent
    ? firstAvatarUrl(persona.avatarUrl, profileQuery.data?.avatarUrl)
    : persona.avatarUrl;
  const friendlyError = agent
    ? friendlyAgentLastError(agent.lastError, agent.lastErrorCode)?.copy
    : null;
  const opensRuntimeTab = Boolean(agent && friendlyError && !isActive);

  return (
    <AgentIdentityCard
      actions={actions}
      ariaLabel={`${title} agent profile`}
      avatar={
        agent ? (
          <AgentRuntimeAvatarControl
            activeTestId={`agent-runtime-active-${agent.pubkey}`}
            avatarUrl={avatarUrl}
            errorLabel={friendlyError}
            errorTestId={`agent-runtime-error-${agent.pubkey}`}
            isActive={isActive}
            isStarting={startingAgentPubkey === agent.pubkey}
            label={title}
            startTestId={`agent-runtime-start-${agent.pubkey}`}
            onOpenError={() => {
              onOpenAgentProfile(agent.pubkey, { tab: "runtime" });
            }}
            onStart={() => onStartAgent(agent.pubkey)}
          />
        ) : (
          <AgentRuntimeAvatarControl
            activeTestId={`persona-runtime-active-${persona.id}`}
            avatarUrl={avatarUrl}
            isActive={false}
            isStarting={startingPersonaIds.has(persona.id)}
            label={title}
            startTestId={`persona-runtime-start-${persona.id}`}
            onStart={() => onStartPersona(persona)}
          />
        )
      }
      avatarUrl={avatarUrl}
      dataTestId={`persona-agent-row-${persona.id}`}
      label={title}
      modelLabel={modelLabel}
      onClick={() => {
        if (agent) {
          onOpenAgentProfile(
            agent.pubkey,
            opensRuntimeTab ? { tab: "runtime" } : undefined,
          );
          return;
        }
        onOpenPersonaProfile(persona);
      }}
      statusBadge={
        agent?.needsRestart ? (
          <Badge className="gap-1" variant="warning">
            <RefreshCw className="h-3 w-3" />
            Restart required
          </Badge>
        ) : null
      }
    />
  );
}

function StandaloneAgentCard({
  agent,
  defaultModel,
  startingAgentPubkey,
  onOpenAgentProfile,
  onStartAgent,
}: {
  agent: ManagedAgent;
  defaultModel: string;
  startingAgentPubkey: string | null;
  onOpenAgentProfile: (
    pubkey: string,
    options?: ProfilePanelOpenOptions,
  ) => void;
  onStartAgent: (pubkey: string) => void;
}) {
  const title = agent.name;
  const profileQuery = useUserProfileQuery(agent.pubkey);
  const friendlyError = friendlyAgentLastError(
    agent.lastError,
    agent.lastErrorCode,
  )?.copy;
  const isActive = isManagedAgentActive(agent);
  const opensRuntimeTab = Boolean(friendlyError && !isActive);

  return (
    <AgentIdentityCard
      ariaLabel={`${title} agent profile`}
      avatar={
        <AgentRuntimeAvatarControl
          activeTestId={`agent-runtime-active-${agent.pubkey}`}
          avatarUrl={profileQuery.data?.avatarUrl}
          errorLabel={friendlyError}
          errorTestId={`agent-runtime-error-${agent.pubkey}`}
          isActive={isActive}
          isStarting={startingAgentPubkey === agent.pubkey}
          label={title}
          startTestId={`agent-runtime-start-${agent.pubkey}`}
          onOpenError={() => {
            onOpenAgentProfile(agent.pubkey, { tab: "runtime" });
          }}
          onStart={() => onStartAgent(agent.pubkey)}
        />
      }
      avatarUrl={profileQuery.data?.avatarUrl}
      dataTestId={`managed-agent-${agent.pubkey}`}
      label={title}
      modelLabel={
        agent.model?.trim()
          ? formatAgentModelLabel(agent.model)
          : formatDefaultModelLabel(defaultModel)
      }
      onClick={() => {
        onOpenAgentProfile(
          agent.pubkey,
          opensRuntimeTab ? { tab: "runtime" } : undefined,
        );
      }}
      statusBadge={
        agent.needsRestart ? (
          <Badge className="gap-1" variant="warning">
            <RefreshCw className="h-3 w-3" />
            Restart required
          </Badge>
        ) : null
      }
    />
  );
}

function formatDefaultModelLabel(defaultModel: string) {
  const model = defaultModel.trim();
  return model ? `Default model (${model})` : "Default model";
}

function firstAvatarUrl(
  ...candidates: Array<string | null | undefined>
): string | null {
  for (const candidate of candidates) {
    const trimmed = candidate?.trim();
    if (trimmed) return trimmed;
  }
  return null;
}

function NewAgentCard({
  canChooseCatalog,
  isManagedHive,
  isPersonasPending,
  openFilePicker,
  onChooseCatalog,
  onCreatePersona,
  onRefreshCompanyAgents,
}: {
  canChooseCatalog: boolean;
  isManagedHive: boolean;
  isPersonasPending: boolean;
  openFilePicker: () => void;
  onChooseCatalog: () => void;
  onCreatePersona: () => void;
  onRefreshCompanyAgents: () => void;
}) {
  const label = isManagedHive ? "Import from company VM" : "New agent";
  return (
    <DropdownMenu modal={false}>
      <DropdownMenuTrigger asChild>
        <CreateIdentityCard
          ariaLabel={label}
          dataTestId="new-agent-card"
          label={label}
        />
      </DropdownMenuTrigger>
      <DropdownMenuContent
        align="start"
        onCloseAutoFocus={(event) => event.preventDefault()}
      >
        {isManagedHive ? (
          <DropdownMenuItem onClick={onRefreshCompanyAgents}>
            Refresh registered VM agents
          </DropdownMenuItem>
        ) : (
          <>
            <DropdownMenuItem
              disabled={isPersonasPending}
              onClick={onCreatePersona}
            >
              Create from scratch
            </DropdownMenuItem>
            {canChooseCatalog ? (
              <DropdownMenuItem
                disabled={isPersonasPending}
                onClick={onChooseCatalog}
              >
                Choose from catalog
              </DropdownMenuItem>
            ) : null}
            <DropdownMenuItem
              data-testid="import-agent-snapshot-menu-item"
              onClick={openFilePicker}
            >
              Import agent snapshot
            </DropdownMenuItem>
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}

function CompanyVmAgentsSection({
  agents,
  error,
  isLoading,
  onOpenAgentProfile,
}: {
  agents: RelayAgent[];
  error: Error | null;
  isLoading: boolean;
  onOpenAgentProfile: (
    pubkey: string,
    options?: ProfilePanelOpenOptions,
  ) => void;
}) {
  return (
    <div className={`${AGENT_CARD_COLUMN_CLASS} space-y-2`}>
      <div className="px-1">
        <h2 className="text-sm font-medium">Company VM agents</h2>
        <p className="text-xs text-muted-foreground">
          Registered public Hive identities from your company VMs. Runtime setup
          remains in Hermes.
        </p>
      </div>
      {isLoading ? (
        <LoadingSkeleton />
      ) : error ? (
        <p className="px-1 text-sm text-destructive">
          Company VM agents could not be loaded. Refresh the registered VM
          agents to try again.
        </p>
      ) : agents.length > 0 ? (
        <div className={AGENT_CARD_GRID_CLASS}>
          {agents.map((agent) => (
            <AgentIdentityCard
              ariaLabel={`${agent.name} company VM agent profile`}
              dataTestId={`company-vm-agent-${agent.pubkey}`}
              key={agent.pubkey}
              label={agent.name}
              modelLabel={`${agent.agentType} · ${agent.status}`}
              onClick={() => onOpenAgentProfile(agent.pubkey)}
              statusBadge={
                <Badge className="mt-1 w-fit" variant="outline">
                  VM hosted
                </Badge>
              }
            />
          ))}
        </div>
      ) : (
        <p className="px-1 text-sm text-muted-foreground">
          No registered company VM agent identity is available yet. Register the
          profile on its VM, then refresh.
        </p>
      )}
    </div>
  );
}

function LoadingSkeleton() {
  return (
    <div className={AGENT_CARD_GRID_CLASS}>
      <IdentityCardSkeleton
        footerSubtitleWidthClass="w-14"
        footerTitleWidthClass="w-24"
      />
      <IdentityCardSkeleton
        footerSubtitleWidthClass="w-20"
        footerTitleWidthClass="w-32"
      />
      <IdentityCardSkeleton
        footerSubtitleWidthClass="w-16"
        footerTitleWidthClass="w-28"
      />
    </div>
  );
}

function CollapsibleAgentGroup({
  groupKey,
  label,
  agents,
  collapsed,
  defaultModel,
  startingAgentPubkey,
  onToggle,
  onOpenAgentProfile,
  onStartAgent,
}: {
  groupKey: string;
  label: string;
  agents: ManagedAgent[];
  collapsed: ReadonlySet<string>;
  defaultModel: string;
  startingAgentPubkey: string | null;
  onToggle: (key: string) => void;
  onOpenAgentProfile: (
    pubkey: string,
    options?: ProfilePanelOpenOptions,
  ) => void;
  onStartAgent: (pubkey: string) => void;
}) {
  const isCollapsed = collapsed.has(groupKey);
  return (
    <div className={`${AGENT_CARD_COLUMN_CLASS} space-y-2`}>
      <button
        className="group flex items-center gap-2 rounded-md px-1 py-1 text-left transition-colors hover:bg-muted/50"
        onClick={() => onToggle(groupKey)}
        type="button"
      >
        {isCollapsed ? (
          <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
        ) : (
          <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <span className="text-sm font-medium">{label}</span>
        <span className="text-xs text-muted-foreground">({agents.length})</span>
      </button>
      {!isCollapsed ? (
        <div className={AGENT_CARD_GRID_CLASS}>
          {agents.map((agent) => (
            <StandaloneAgentCard
              agent={agent}
              defaultModel={defaultModel}
              key={agent.pubkey}
              startingAgentPubkey={startingAgentPubkey}
              onOpenAgentProfile={onOpenAgentProfile}
              onStartAgent={onStartAgent}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}
