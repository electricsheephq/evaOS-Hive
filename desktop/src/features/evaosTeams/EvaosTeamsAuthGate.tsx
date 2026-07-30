import { isTauri } from "@tauri-apps/api/core";
import { type ReactNode, useCallback, useEffect, useState } from "react";

import { ThemeGrainientBackground } from "@/app/ThemeGrainientBackground";
import { useCommunities } from "@/features/communities/useCommunities";
import {
  evaosTeamsRefreshDelay,
  evaosTeamsStatusCopy,
  getEvaosTeamsAuthStatus,
  startEvaosTeamsLogin,
  type EvaosTeamsAuthStatus,
} from "@/features/evaosTeams/api";
import { isManagedCommunityStateReady } from "@/features/evaosTeams/managedCommunity";
import { ProductMark } from "@/shared/product/ProductMark";
import { Button } from "@/shared/ui/button";
import { StartupWindowDragRegion } from "@/shared/ui/StartupWindowDragRegion";

export function EvaosTeamsAuthGate({ children }: { children: ReactNode }) {
  const tauri = isTauri();
  const { communities, activeCommunityId, reconcileManagedCommunity } =
    useCommunities();
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

  const activeEntitlement =
    status?.phase === "active" ? status.entitlement : undefined;
  let managedCommunityReady = false;
  let managedCommunityError: string | null = null;
  if (activeEntitlement) {
    try {
      managedCommunityReady = isManagedCommunityStateReady(
        communities,
        activeCommunityId,
        activeEntitlement,
      );
    } catch (communityError) {
      managedCommunityError =
        communityError instanceof Error
          ? communityError.message
          : String(communityError);
    }
  }

  const reconcileActiveCommunity = useCallback(() => {
    if (!activeEntitlement) return;
    try {
      if (!reconcileManagedCommunity(activeEntitlement)) {
        setError("Hive could not save the selected company community.");
      }
    } catch (communityError) {
      setError(
        communityError instanceof Error
          ? communityError.message
          : String(communityError),
      );
    }
  }, [activeEntitlement, reconcileManagedCommunity]);

  useEffect(() => {
    if (!activeEntitlement || managedCommunityReady || managedCommunityError) {
      return;
    }
    reconcileActiveCommunity();
  }, [
    activeEntitlement,
    managedCommunityError,
    managedCommunityReady,
    reconcileActiveCommunity,
  ]);

  async function run(action: () => Promise<EvaosTeamsAuthStatus>) {
    setWorking(true);
    setError(null);
    try {
      setStatus(await action());
    } catch (actionError) {
      const actionMessage =
        actionError instanceof Error
          ? actionError.message
          : String(actionError);
      await refresh();
      setError(actionMessage);
    } finally {
      setWorking(false);
    }
  }

  if (
    !tauri ||
    (status && !status.managed) ||
    (activeEntitlement && managedCommunityReady)
  ) {
    return children;
  }

  const copy =
    activeEntitlement && !managedCommunityReady
      ? {
          title: "Opening your Hive community",
          body: "Electric Sheep verified this device. Hive is selecting the authorized company community.",
        }
      : status
        ? evaosTeamsStatusCopy(status)
        : {
            title: "Checking managed access",
            body: "Hive is checking your Electric Sheep session.",
          };
  const maySignIn =
    status?.phase === "signed_out" || status?.phase === "reauth_required";
  const mayRetry =
    status?.phase === "keychain_locked" || status?.phase === "logout_pending";

  return (
    <main className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-background px-6 py-12">
      <StartupWindowDragRegion />
      <ThemeGrainientBackground />
      <section className="relative z-10 w-full max-w-md rounded-2xl border border-border/70 bg-background/90 p-7 shadow-2xl backdrop-blur-xl">
        <ProductMark
          className="mb-6 h-11 w-auto text-foreground"
          imageClassName="mb-6 h-16 w-16 rounded-[22%]"
        />
        <p className="mb-2 text-xs font-semibold uppercase tracking-[0.18em] text-muted-foreground">
          Hive
        </p>
        <h1 className="text-2xl font-semibold text-foreground">{copy.title}</h1>
        <p className="mt-3 text-sm leading-6 text-muted-foreground">
          {copy.body}
        </p>

        {error || managedCommunityError ? (
          <p
            className="mt-5 rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
            role="alert"
          >
            {error ?? managedCommunityError}
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
          {mayRetry ? (
            <Button
              disabled={working}
              variant="outline"
              onClick={() => void run(async () => (await refresh()) ?? status)}
            >
              Try again
            </Button>
          ) : null}
          {activeEntitlement && !managedCommunityReady ? (
            <Button
              disabled={working || Boolean(managedCommunityError)}
              variant="outline"
              onClick={reconcileActiveCommunity}
            >
              Try again
            </Button>
          ) : null}
        </div>
      </section>
    </main>
  );
}
