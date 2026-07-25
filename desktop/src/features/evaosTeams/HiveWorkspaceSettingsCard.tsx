import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  addHiveRoomParticipant,
  getHiveCollaborationState,
  hiveCollaborationQueryKey,
  inviteHiveMember,
} from "@/features/evaosTeams/api";
import { useEvaosTeamsAuthority } from "@/features/evaosTeams/authority";
import { invalidateManagedWorkspaceAgentProjection } from "@/features/evaosTeams/managedWorkspaceAgents";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";

export function HiveWorkspaceSettingsCard() {
  const authority = useEvaosTeamsAuthority();
  const queryClient = useQueryClient();
  const [email, setEmail] = useState("");
  const [role, setRole] = useState<"admin" | "employee" | "member">("employee");
  const [roomId, setRoomId] = useState("");
  const [agentPublicKey, setAgentPublicKey] = useState("");
  const [notice, setNotice] = useState<string | null>(null);

  const workspace = useQuery({
    queryKey: hiveCollaborationQueryKey,
    queryFn: getHiveCollaborationState,
    enabled: authority.managed,
    refetchInterval: 30_000,
  });
  const canManage =
    workspace.data?.role === "owner" || workspace.data?.role === "admin";
  const streamRooms = useMemo(
    () =>
      (workspace.data?.rooms ?? []).filter(
        (room) => (room.channelType ?? "stream") === "stream",
      ),
    [workspace.data?.rooms],
  );

  const invite = useMutation({
    mutationFn: () => inviteHiveMember(email, role),
    onSuccess: (result) => {
      setEmail("");
      setNotice(
        result.emailDispatchStatus === "sent"
          ? "Invitation sent."
          : "Invitation created, but email delivery needs support follow-up.",
      );
      void queryClient.invalidateQueries({
        queryKey: hiveCollaborationQueryKey,
      });
    },
    onError: (error) => {
      setNotice(
        error instanceof Error
          ? error.message
          : "Invitation could not be sent.",
      );
    },
  });

  const assignAgent = useMutation({
    mutationFn: () => addHiveRoomParticipant(roomId, agentPublicKey, "agent"),
    onSuccess: () => {
      setNotice("Agent added to the channel.");
      void invalidateManagedWorkspaceAgentProjection(queryClient);
    },
    onError: (error) => {
      setNotice(
        error instanceof Error
          ? error.message
          : "Agent could not be added to the channel.",
      );
    },
  });

  if (workspace.isPending) {
    return (
      <div className="p-6 text-sm text-muted-foreground">
        Loading Hive workspace…
      </div>
    );
  }
  if (workspace.isError || !workspace.data) {
    return (
      <div className="space-y-3 p-6">
        <p className="text-sm text-destructive">
          Hive workspace access could not be loaded.
        </p>
        <Button onClick={() => void workspace.refetch()} variant="outline">
          Try again
        </Button>
      </div>
    );
  }

  const state = workspace.data;
  return (
    <section className="flex min-h-0 flex-1 flex-col overflow-y-auto p-6">
      <div className="mx-auto w-full max-w-3xl space-y-6">
        <header>
          <h2 className="text-xl font-semibold">Workspace access</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            {state.activeSeats.toLocaleString()} active member
            {state.activeSeats === 1 ? "" : "s"} ·{" "}
            {state.seatLimit.toLocaleString()} seats · relay{" "}
            {state.reconciliationStatus}
          </p>
        </header>

        {notice ? (
          <p className="rounded-md border bg-muted/30 px-3 py-2 text-sm">
            {notice}
          </p>
        ) : null}

        <div className="rounded-lg border">
          <div className="border-b px-4 py-3">
            <h3 className="font-medium">People</h3>
          </div>
          <div className="divide-y">
            {state.members.map((member) => (
              <div
                className="flex items-center justify-between gap-4 px-4 py-3 text-sm"
                key={member.membershipId}
              >
                <div className="min-w-0">
                  <p className="truncate font-medium">{member.displayName}</p>
                  <p className="truncate text-muted-foreground">
                    {member.email}
                  </p>
                </div>
                <div className="shrink-0 text-right">
                  <p className="capitalize">{member.role}</p>
                  <p className="text-xs text-muted-foreground">
                    {member.bindingStatus === "bound"
                      ? "Hive ready"
                      : "Sign in required"}
                  </p>
                </div>
              </div>
            ))}
          </div>
        </div>

        {canManage ? (
          <form
            className="space-y-3 rounded-lg border p-4"
            onSubmit={(event) => {
              event.preventDefault();
              setNotice(null);
              invite.mutate();
            }}
          >
            <div>
              <h3 className="font-medium">Invite a teammate</h3>
              <p className="text-sm text-muted-foreground">
                Seat limits are enforced by the company account.
              </p>
            </div>
            <div className="flex flex-col gap-2 sm:flex-row">
              <Input
                aria-label="Teammate email"
                onChange={(event) => setEmail(event.target.value)}
                placeholder="name@company.com"
                required
                type="email"
                value={email}
              />
              <select
                aria-label="Workspace role"
                className="h-9 rounded-md border bg-background px-3 text-sm"
                onChange={(event) =>
                  setRole(event.target.value as "admin" | "employee" | "member")
                }
                value={role}
              >
                <option value="employee">Employee</option>
                <option value="member">Member</option>
                <option value="admin">Admin</option>
              </select>
              <Button disabled={invite.isPending} type="submit">
                {invite.isPending ? "Sending…" : "Send invite"}
              </Button>
            </div>
          </form>
        ) : null}

        <div className="rounded-lg border">
          <div className="border-b px-4 py-3">
            <h3 className="font-medium">Agents</h3>
            <p className="text-sm text-muted-foreground">
              Only agents registered to this company can join Hive channels.
            </p>
          </div>
          {state.agents.length === 0 ? (
            <p className="px-4 py-4 text-sm text-muted-foreground">
              No company agent is connected to Hive yet.
            </p>
          ) : (
            <div className="divide-y">
              {state.agents.map((agent) => (
                <div
                  className="flex items-center justify-between px-4 py-3 text-sm"
                  key={agent.agentInstanceId}
                >
                  <span className="font-medium">{agent.displayName}</span>
                  <span className="capitalize text-muted-foreground">
                    {agent.runtime}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {canManage && state.agents.length > 0 && streamRooms.length > 0 ? (
          <form
            className="space-y-3 rounded-lg border p-4"
            onSubmit={(event) => {
              event.preventDefault();
              setNotice(null);
              assignAgent.mutate();
            }}
          >
            <h3 className="font-medium">Add an agent to a channel</h3>
            <div className="flex flex-col gap-2 sm:flex-row">
              <select
                aria-label="Channel"
                className="h-9 min-w-0 flex-1 rounded-md border bg-background px-3 text-sm"
                onChange={(event) => setRoomId(event.target.value)}
                required
                value={roomId}
              >
                <option value="">Choose channel</option>
                {streamRooms.map((room) => (
                  <option key={room.roomId} value={room.roomId}>
                    {room.name ?? room.roomId.slice(0, 8)}
                  </option>
                ))}
              </select>
              <select
                aria-label="Agent"
                className="h-9 min-w-0 flex-1 rounded-md border bg-background px-3 text-sm"
                onChange={(event) => setAgentPublicKey(event.target.value)}
                required
                value={agentPublicKey}
              >
                <option value="">Choose agent</option>
                {state.agents.map((agent) => (
                  <option key={agent.agentInstanceId} value={agent.publicKey}>
                    {agent.displayName}
                  </option>
                ))}
              </select>
              <Button disabled={assignAgent.isPending} type="submit">
                {assignAgent.isPending ? "Adding…" : "Add agent"}
              </Button>
            </div>
          </form>
        ) : null}
      </div>
    </section>
  );
}
