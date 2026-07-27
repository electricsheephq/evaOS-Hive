import type { ManagedAgent, RelayAgent } from "@/shared/api/types";

export type ProfileActivityAgent = Pick<
  ManagedAgent,
  "pubkey" | "name" | "status"
> & {
  avatarUrl?: string | null;
};

export function resolveProfileActivityAgent({
  effectivePubkey,
  isBot,
  managedAgent,
  profile,
  relayAgent,
  viewerIsOwner,
}: {
  effectivePubkey: string | null;
  isBot: boolean;
  managedAgent: ManagedAgent | undefined;
  profile: { avatarUrl?: string | null; displayName?: string | null } | null;
  relayAgent: RelayAgent | undefined;
  viewerIsOwner: boolean;
}): ProfileActivityAgent | null {
  if (managedAgent) {
    return {
      avatarUrl: managedAgent.avatarUrl,
      name: managedAgent.name,
      pubkey: managedAgent.pubkey,
      status: managedAgent.status,
    };
  }

  if (!viewerIsOwner || !effectivePubkey || !isBot) {
    return null;
  }

  return {
    avatarUrl: profile?.avatarUrl ?? null,
    name: relayAgent?.name ?? profile?.displayName?.trim() ?? "Agent",
    pubkey: effectivePubkey,
    // `deployed` here enables the owner's observer/activity feed; it is not a
    // liveness claim. Relay presence remains the status authority displayed by
    // agent surfaces, while an unknown remote agent can still have durable
    // activity history and later frames for its owner. Explicit offline stays
    // stopped.
    status: relayAgent?.status === "offline" ? "stopped" : "deployed",
  };
}
