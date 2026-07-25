import * as React from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bot, RefreshCw } from "lucide-react";

import {
  addHiveRoomParticipant,
  getHiveCollaborationState,
  hiveCollaborationQueryKey,
} from "@/features/evaosTeams/api";
import { Button } from "@/shared/ui/button";

export function ManagedAgentsView() {
  const queryClient = useQueryClient();
  const [roomId, setRoomId] = React.useState("");
  const [agentPublicKey, setAgentPublicKey] = React.useState("");
  const [notice, setNotice] = React.useState<string | null>(null);
  const workspace = useQuery({
    queryKey: hiveCollaborationQueryKey,
    queryFn: getHiveCollaborationState,
    refetchInterval: 30_000,
  });
  const assignAgent = useMutation({
    mutationFn: () => addHiveRoomParticipant(roomId, agentPublicKey, "agent"),
    onSuccess: () => {
      setNotice("Agent added to the channel.");
      void queryClient.invalidateQueries({
        queryKey: hiveCollaborationQueryKey,
      });
    },
    onError: (error) => {
      setNotice(
        error instanceof Error ? error.message : "Agent could not be added.",
      );
    },
  });

  if (workspace.isPending) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center text-sm text-muted-foreground">
        Loading company agents…
      </div>
    );
  }

  if (workspace.isError || !workspace.data) {
    return (
      <div className="flex min-h-0 flex-1 items-center justify-center p-6">
        <div className="space-y-3 text-center">
          <p className="text-sm text-destructive">
            Company agents could not be loaded.
          </p>
          <Button onClick={() => void workspace.refetch()} variant="outline">
            <RefreshCw className="size-4" />
            Try again
          </Button>
        </div>
      </div>
    );
  }

  const state = workspace.data;
  const canAssign = state.role === "owner" || state.role === "admin";
  const rooms = state.rooms.filter((room) => room.channelType !== "dm");

  return (
    <section className="min-h-0 flex-1 overflow-y-auto p-6">
      <div className="mx-auto w-full max-w-5xl space-y-6">
        <header>
          <h1 className="text-2xl font-semibold">Agents</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Hermes agents registered to this Hive workspace. Provider setup,
            memory, tools, and permissions stay inside Hermes.
          </p>
        </header>

        {notice ? (
          <p className="rounded-md border bg-muted/30 px-3 py-2 text-sm">
            {notice}
          </p>
        ) : null}

        {state.agents.length === 0 ? (
          <div className="rounded-xl border border-dashed p-8 text-center">
            <Bot className="mx-auto size-8 text-muted-foreground" />
            <h2 className="mt-3 font-medium">No company agent connected</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              An Electric Sheep-local Hermes identity must be registered before
              an agent can appear or join a channel.
            </p>
          </div>
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {state.agents.map((agent) => {
              const assignedRooms = rooms.filter((room) =>
                room.agentInstances.includes(agent.agentInstanceId),
              );
              return (
                <article
                  className="rounded-xl border p-4"
                  key={agent.agentInstanceId}
                >
                  <div className="flex items-center gap-3">
                    <div className="flex size-10 items-center justify-center rounded-full bg-muted">
                      <Bot className="size-5" />
                    </div>
                    <div className="min-w-0">
                      <h2 className="truncate font-medium">
                        {agent.displayName}
                      </h2>
                      <p className="text-xs capitalize text-muted-foreground">
                        {agent.runtime}
                      </p>
                    </div>
                  </div>
                  <p className="mt-3 text-xs text-muted-foreground">
                    {assignedRooms.length === 0
                      ? "Not assigned to a channel"
                      : assignedRooms
                          .map(
                            (room) =>
                              `#${room.name ?? room.roomId.slice(0, 8)}`,
                          )
                          .join(" · ")}
                  </p>
                </article>
              );
            })}
          </div>
        )}

        {canAssign && state.agents.length > 0 && rooms.length > 0 ? (
          <form
            className="space-y-3 rounded-xl border p-4"
            onSubmit={(event) => {
              event.preventDefault();
              setNotice(null);
              assignAgent.mutate();
            }}
          >
            <div>
              <h2 className="font-medium">Add an agent to a channel</h2>
              <p className="text-sm text-muted-foreground">
                Only agents registered to this company are available.
              </p>
            </div>
            <div className="flex flex-col gap-2 sm:flex-row">
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
              <select
                aria-label="Channel"
                className="h-9 min-w-0 flex-1 rounded-md border bg-background px-3 text-sm"
                onChange={(event) => setRoomId(event.target.value)}
                required
                value={roomId}
              >
                <option value="">Choose channel</option>
                {rooms.map((room) => (
                  <option key={room.roomId} value={room.roomId}>
                    {room.name ?? room.roomId.slice(0, 8)}
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
