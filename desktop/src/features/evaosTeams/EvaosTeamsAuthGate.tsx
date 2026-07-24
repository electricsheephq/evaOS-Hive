import { isTauri } from "@tauri-apps/api/core";
import { type ReactNode, useCallback, useEffect, useState } from "react";

import {
  evaosTeamsRefreshDelay,
  evaosTeamsStatusCopy,
  getEvaosTeamsAuthStatus,
  logoutEvaosTeams,
  startEvaosTeamsLogin,
  type EvaosTeamsAuthStatus,
} from "@/features/evaosTeams/api";
import {
  createManagedEvaosTeamsAuthority,
  EvaosTeamsAuthorityProvider,
  nativeEvaosTeamsAuthority,
} from "@/features/evaosTeams/authority";
import { ThemeGrainientBackground } from "@/app/ThemeGrainientBackground";
import { Button } from "@/shared/ui/button";
import { BuzzMark } from "@/shared/ui/buzz-logo/BuzzMark";
import { StartupWindowDragRegion } from "@/shared/ui/StartupWindowDragRegion";
import { removeChannelSnapshotForRelay } from "@/features/channels/channelSnapshot";
import { removeMessageSnapshotsForRelay } from "@/features/messages/lib/messageSnapshot";
import { clearSavedCommunitySnapshot } from "@/features/agents/activeAgentTurnsStore";

export function EvaosTeamsAuthGate({ children }: { children: ReactNode }) {
  const tauri = isTauri();
  const [status, setStatus] = useState<EvaosTeamsAuthStatus | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await getEvaosTeamsAuthStatus();
      setStatus(next);
      setError(null);
      return next;
    } catch (refreshError) {
      // A failed managed refresh must unmount the previously authorized tree;
      // stale renderer state can never extend an expired server revision.
      setStatus(null);
      setError(
        refreshError instanceof Error
          ? refreshError.message
          : String(refreshError),
      );
      return null;
    }
  }, []);

  useEffect(() => {
    if (!tauri) return;
    void refresh();
  }, [refresh, tauri]);

  useEffect(() => {
    if (
      !status?.managed ||
      (status.phase !== "active" && status.phase !== "sync_pending")
    )
      return;
    const timer = window.setTimeout(() => {
      void refresh();
    }, evaosTeamsRefreshDelay(status));
    return () => window.clearTimeout(timer);
  }, [refresh, status]);

  useEffect(() => {
    const entitlement =
      status?.phase === "active" ? status.entitlement : undefined;
    if (!entitlement) return;
    const relay = new URL(entitlement.relayHost);
    relay.protocol = "wss:";
    const relayUrl = relay.toString().replace(/\/$/, "");
    removeChannelSnapshotForRelay(relayUrl);
    removeMessageSnapshotsForRelay(relayUrl);
    clearSavedCommunitySnapshot(entitlement.communityId);
  }, [status]);

  async function run(action: () => Promise<EvaosTeamsAuthStatus>) {
    setWorking(true);
    setError(null);
    try {
      setStatus(await action());
    } catch (actionError) {
      setError(
        actionError instanceof Error
          ? actionError.message
          : String(actionError),
      );
      await refresh();
    } finally {
      setWorking(false);
    }
  }

  if (!tauri || (status && !status.managed)) {
    return (
      <EvaosTeamsAuthorityProvider authority={nativeEvaosTeamsAuthority}>
        {children}
      </EvaosTeamsAuthorityProvider>
    );
  }
  if (status?.phase === "active" && status.entitlement) {
    return (
      <EvaosTeamsAuthorityProvider
        authority={createManagedEvaosTeamsAuthority(status.entitlement)}
      >
        {children}
      </EvaosTeamsAuthorityProvider>
    );
  }
  const copy = status
    ? evaosTeamsStatusCopy(status)
    : {
        title: "Checking managed access",
        body: "evaOS Teams is checking Keychain and your ElectricSheep access.",
      };
  const entitlement = status?.entitlement;
  const maySignIn =
    status?.phase === "signed_out" || status?.phase === "reauth_required";

  return (
    <main className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-background px-6 py-12">
      <StartupWindowDragRegion />
      <ThemeGrainientBackground />
      <section className="relative z-10 w-full max-w-md rounded-2xl border border-border/70 bg-background/90 p-7 shadow-2xl backdrop-blur-xl">
        <BuzzMark className="mb-6 h-11 w-auto text-foreground" />
        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
          evaOS Teams
        </p>
        <h1 className="text-2xl font-semibold text-foreground">{copy.title}</h1>
        <p className="mt-3 text-sm leading-6 text-muted-foreground">
          {copy.body}
        </p>

        {entitlement ? (
          <dl className="mt-6 space-y-3 rounded-xl border border-border bg-muted/30 p-4 text-sm">
            <div>
              <dt className="text-xs uppercase tracking-wide text-muted-foreground">
                Relay selected by ElectricSheep
              </dt>
              <dd className="mt-1 break-all font-mono text-foreground">
                {entitlement.relayHost}
              </dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">Role</dt>
              <dd className="font-medium text-foreground">
                {entitlement.role}
              </dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">Access revision</dt>
              <dd className="font-medium text-foreground">
                {entitlement.accessRevision}
              </dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">Agent assignment</dt>
              <dd className="font-medium text-foreground">
                {entitlement.assignmentStatus}
              </dd>
            </div>
            <div className="flex justify-between gap-4">
              <dt className="text-muted-foreground">Relay projection</dt>
              <dd className="font-medium text-foreground">
                {entitlement.reconciliationStatus}
              </dd>
            </div>
          </dl>
        ) : null}

        {error ? (
          <p
            className="mt-5 rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
            role="alert"
          >
            {error}
          </p>
        ) : null}

        <div className="mt-6 flex flex-col gap-3">
          {maySignIn ? (
            <Button
              disabled={working}
              onClick={() => void run(startEvaosTeamsLogin)}
            >
              {working
                ? "Waiting for browser sign-in…"
                : "Sign in with ElectricSheep"}
            </Button>
          ) : null}
          {status?.phase === "sync_pending" ? (
            <Button
              disabled={working}
              variant="outline"
              onClick={() => void run(logoutEvaosTeams)}
            >
              Sign out
            </Button>
          ) : null}
          {status?.phase === "keychain_locked" ||
          status?.phase === "logout_pending" ? (
            <Button
              disabled={working}
              variant="outline"
              onClick={() => void run(async () => (await refresh()) ?? status)}
            >
              Try again
            </Button>
          ) : null}
        </div>

        <p className="mt-6 text-xs leading-5 text-muted-foreground">
          Provider setup and agent permissions remain inside Hermes. This screen
          verifies only your ElectricSheep Teams membership and managed relay
          identity.
        </p>
      </section>
    </main>
  );
}
