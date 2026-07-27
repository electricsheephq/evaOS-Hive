import * as React from "react";

import type {
  HiveCompanyAgentPolicy,
  HiveCompanyMember,
} from "@/features/evaosTeams/api";
import {
  useHiveCompanyAgentPolicyQuery,
  useSetHiveCompanyAgentPolicyMutation,
} from "@/features/evaosTeams/hooks";
import type { CompanyVmAgent } from "@/features/agents/lib/companyAgentCatalog";
import {
  availablePolicyMembers,
  type CompanyAgentRoomOption,
} from "@/features/agents/lib/companyAgentResponderPolicy";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Checkbox } from "@/shared/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";

type Props = {
  agent: CompanyVmAgent;
  members: readonly HiveCompanyMember[];
  rooms: readonly CompanyAgentRoomOption[];
  onOpenChange: (open: boolean) => void;
  open: boolean;
};

export function CompanyAgentResponderPolicyDialog({
  agent,
  members,
  rooms,
  onOpenChange,
  open,
}: Props) {
  const policyQuery = useHiveCompanyAgentPolicyQuery({
    agentInstanceId: agent.agentInstanceId,
    enabled: open,
  });
  const mutation = useSetHiveCompanyAgentPolicyMutation();
  const [selectedRoomIds, setSelectedRoomIds] = React.useState<string[]>([]);
  const [selectedMemberIds, setSelectedMemberIds] = React.useState<string[]>(
    [],
  );
  const [dirty, setDirty] = React.useState(false);
  const availableMembers = React.useMemo(
    () => availablePolicyMembers(members),
    [members],
  );

  React.useEffect(() => {
    if (!open || dirty || !policyQuery.data) return;
    setSelectedRoomIds(policyQuery.data.allowedRoomIds);
    setSelectedMemberIds(policyQuery.data.allowedAuthorMembershipIds);
  }, [dirty, open, policyQuery.data]);

  function toggle(
    value: string,
    selected: string[],
    setSelected: React.Dispatch<React.SetStateAction<string[]>>,
  ) {
    setDirty(true);
    setSelected(
      selected.includes(value)
        ? selected.filter((candidate) => candidate !== value)
        : [...selected, value],
    );
  }

  async function save(policy: HiveCompanyAgentPolicy) {
    await mutation.mutateAsync({
      agentInstanceId: agent.agentInstanceId,
      expectedRevision: policy.desiredRevision,
      allowedRoomIds: rooms
        .filter((room) => selectedRoomIds.includes(room.id))
        .map((room) => room.id),
      allowedAuthorMembershipIds: availableMembers
        .filter((member) => selectedMemberIds.includes(member.membershipId))
        .map((member) => member.membershipId),
    });
    setDirty(false);
  }

  const policy = policyQuery.data;
  const feedback = policy ? policyFeedback(policy) : null;

  return (
    <Dialog onOpenChange={onOpenChange} open={open}>
      <DialogContent className="max-w-2xl overflow-hidden p-0">
        <div className="flex max-h-[85vh] flex-col">
          <DialogHeader className="shrink-0 border-b border-border/60 px-6 py-5 pr-14">
            <DialogTitle>Who can talk to {agent.name}</DialogTitle>
            <DialogDescription>
              Choose existing Hive rooms and company members. Runtime setup,
              model, memory, tools, and identity remain in Hermes.
            </DialogDescription>
          </DialogHeader>

          <div className="min-h-0 flex-1 space-y-6 overflow-y-auto px-6 py-5">
            {policyQuery.isLoading ? (
              <p className="text-sm text-muted-foreground">Loading policy…</p>
            ) : policyQuery.error instanceof Error ? (
              <p className="text-sm text-destructive">
                {policyQuery.error.message}
              </p>
            ) : policy ? (
              <>
                {feedback ? (
                  <div className="flex items-center gap-2 rounded-xl border border-border/60 px-3 py-2 text-sm">
                    <Badge variant="outline">{feedback.label}</Badge>
                    <span className="text-muted-foreground">
                      {feedback.copy}
                    </span>
                  </div>
                ) : null}

                <PolicyOptions
                  emptyCopy="Add this agent to a native Hive channel before selecting it here."
                  label="Allowed rooms"
                  options={rooms.map((room) => ({
                    id: room.id,
                    label: `#${room.name}`,
                    detail: room.visibility,
                  }))}
                  selected={selectedRoomIds}
                  onToggle={(id) =>
                    toggle(id, selectedRoomIds, setSelectedRoomIds)
                  }
                />

                <PolicyOptions
                  emptyCopy="No active company member identity is available."
                  label="Allowed people"
                  options={availableMembers.map((member) => ({
                    id: member.membershipId,
                    label: member.displayName,
                    detail: "company member",
                  }))}
                  selected={selectedMemberIds}
                  onToggle={(id) =>
                    toggle(id, selectedMemberIds, setSelectedMemberIds)
                  }
                />
              </>
            ) : null}
          </div>

          <div className="flex shrink-0 justify-end gap-2 border-t border-border/60 px-6 py-4">
            <Button onClick={() => onOpenChange(false)} variant="outline">
              Close
            </Button>
            <Button
              disabled={!dirty || !policy || mutation.isPending}
              onClick={() => {
                if (policy) void save(policy);
              }}
            >
              {mutation.isPending ? "Saving…" : "Save access"}
            </Button>
          </div>
          {mutation.error instanceof Error ? (
            <p className="shrink-0 px-6 pb-4 text-sm text-destructive">
              {mutation.error.message}
            </p>
          ) : null}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function PolicyOptions({
  emptyCopy,
  label,
  options,
  selected,
  onToggle,
}: {
  emptyCopy: string;
  label: string;
  options: { detail: string; id: string; label: string }[];
  selected: readonly string[];
  onToggle: (id: string) => void;
}) {
  return (
    <fieldset className="space-y-2">
      <legend className="text-sm font-medium">{label}</legend>
      {options.length === 0 ? (
        <p className="text-sm text-muted-foreground">{emptyCopy}</p>
      ) : (
        <div className="grid gap-2 sm:grid-cols-2">
          {options.map((option) => (
            <label
              className="flex cursor-pointer items-center gap-3 rounded-xl border border-border/60 px-3 py-3"
              htmlFor={`company-agent-policy-${option.id}`}
              key={option.id}
            >
              <Checkbox
                checked={selected.includes(option.id)}
                id={`company-agent-policy-${option.id}`}
                onCheckedChange={() => onToggle(option.id)}
              />
              <span className="min-w-0">
                <span className="block truncate text-sm font-medium">
                  {option.label}
                </span>
                <span className="block text-xs text-muted-foreground">
                  {option.detail}
                </span>
              </span>
            </label>
          ))}
        </div>
      )}
    </fieldset>
  );
}

function policyFeedback(policy: HiveCompanyAgentPolicy) {
  if (
    policy.status === "applied" &&
    policy.appliedRevision === policy.desiredRevision
  ) {
    return {
      label: "Applied",
      copy: `Revision ${policy.appliedRevision} is active on the VM.`,
    };
  }
  if (policy.status === "error") {
    return {
      label: "Needs attention",
      copy: policy.lastErrorCode
        ? `The VM rejected revision ${policy.desiredRevision} (${policy.lastErrorCode}).`
        : `The VM rejected revision ${policy.desiredRevision}.`,
    };
  }
  return {
    label: "Waiting for VM",
    copy: `Revision ${policy.desiredRevision} is saved but not yet acknowledged by the VM bridge.`,
  };
}
