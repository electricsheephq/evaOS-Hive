import { isTauri } from "@tauri-apps/api/core";
import { type ReactNode, useCallback, useEffect, useState } from "react";

import {
  evaosTeamsRefreshDelay,
  evaosTeamsStatusCopy,
  getEvaosTeamsAuthStatus,
  startEvaosTeamsLogin,
  type EvaosTeamsAuthStatus,
} from "@/features/evaosTeams/api";
import { ThemeGrainientBackground } from "@/app/ThemeGrainientBackground";
import { Button } from "@/shared/ui/button";
import { BuzzMark } from "@/shared/ui/buzz-logo/BuzzMark";
import { StartupWindowDragRegion } from "@/shared/ui/StartupWindowDragRegion";

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
    if (!status?.managed || status.phase !== "active") return;
    const timer = window.setTimeout(() => {
      void refresh();
    }, evaosTeamsRefreshDelay(status));
    return () => window.clearTimeout(timer);
  }, [refresh, status]);

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

  if (
    !tauri ||
    (status && !status.managed) ||
    (status?.phase === "active" && status.entitlement)
  )
    return children;
  const copy = status
    ? evaosTeamsStatusCopy(status)
    : {
        title: "Checking managed access",
        body: "Hive is checking Keychain and your Electric Sheep access.",
      };
  const maySignIn =
    status?.phase === "signed_out" || status?.phase === "reauth_required";

  return (
    <main className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-background px-6 py-12">
      <StartupWindowDragRegion />
      <ThemeGrainientBackground />
      <section className="relative z-10 w-full max-w-md rounded-2xl border border-border/70 bg-background/90 p-7 shadow-2xl backdrop-blur-xl">
        <BuzzMark className="mb-6 h-11 w-auto text-foreground" />
        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
          Hive
        </p>
        <h1 className="text-2xl font-semibold text-foreground">{copy.title}</h1>
        <p className="mt-3 text-sm leading-6 text-muted-foreground">
          {copy.body}
        </p>

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
                : "Sign in with Electric Sheep"}
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
          verifies only your Electric Sheep membership and managed relay
          identity.
        </p>
      </section>
    </main>
  );
}
