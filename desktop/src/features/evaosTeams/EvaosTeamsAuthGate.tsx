import { isTauri } from "@tauri-apps/api/core";
import {
  type FormEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useState,
} from "react";

import {
  evaosTeamsRefreshDelay,
  evaosTeamsStatusCopy,
  getEvaosTeamsAuthStatus,
  startEvaosTeamsLogin,
  submitEvaosTeamsLoginCode,
  type EvaosTeamsAuthStatus,
} from "@/features/evaosTeams/api";
import { ThemeGrainientBackground } from "@/app/ThemeGrainientBackground";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { ProductMark } from "@/shared/product/ProductMark";
import { StartupWindowDragRegion } from "@/shared/ui/StartupWindowDragRegion";
import { useCommunities } from "@/features/communities/useCommunities";
import { isManagedCommunityStateReady } from "@/features/evaosTeams/managedCommunity";

export function EvaosTeamsAuthGate({ children }: { children: ReactNode }) {
  const tauri = isTauri();
  const { communities, activeCommunityId, reconcileManagedCommunity } =
    useCommunities();
  const [status, setStatus] = useState<EvaosTeamsAuthStatus | null>(null);
  const [working, setWorking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loginPending, setLoginPending] = useState(false);
  const [backupCode, setBackupCode] = useState("");
  const [backupCodeSent, setBackupCodeSent] = useState(false);
  const [submittingCode, setSubmittingCode] = useState(false);

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
    if (!activeEntitlement || managedCommunityReady || managedCommunityError)
      return;
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

  async function runLogin() {
    setLoginPending(true);
    setBackupCode("");
    setBackupCodeSent(false);
    try {
      await run(startEvaosTeamsLogin);
    } finally {
      setLoginPending(false);
      setBackupCode("");
      setBackupCodeSent(false);
    }
  }

  async function submitBackupCode(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!backupCode.trim() || submittingCode || backupCodeSent) return;
    setSubmittingCode(true);
    setError(null);
    try {
      await submitEvaosTeamsLoginCode(backupCode);
      setBackupCodeSent(true);
    } catch (submitError) {
      setError(
        submitError instanceof Error
          ? submitError.message
          : String(submitError),
      );
    } finally {
      setSubmittingCode(false);
    }
  }

  if (
    !tauri ||
    (status && !status.managed) ||
    (activeEntitlement && managedCommunityReady)
  )
    return children;
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
            body: "Hive is checking Keychain and your Electric Sheep access.",
          };
  const maySignIn =
    status?.phase === "signed_out" || status?.phase === "reauth_required";

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
            <Button disabled={working} onClick={() => void runLogin()}>
              {working
                ? "Waiting for browser sign-in…"
                : "Sign in with Electric Sheep"}
            </Button>
          ) : null}
          {loginPending ? (
            <form className="flex flex-col gap-2" onSubmit={submitBackupCode}>
              <Input
                aria-label="Hive backup code"
                autoCapitalize="characters"
                autoComplete="one-time-code"
                disabled={backupCodeSent}
                inputMode="text"
                onChange={(event) => setBackupCode(event.target.value)}
                placeholder="Enter backup code"
                spellCheck={false}
                value={backupCode}
              />
              <Button
                disabled={
                  !backupCode.trim() || submittingCode || backupCodeSent
                }
                type="submit"
                variant="outline"
              >
                {backupCodeSent
                  ? "Code sent — finishing sign-in…"
                  : submittingCode
                    ? "Checking code…"
                    : "Use backup code"}
              </Button>
              <p className="text-xs leading-5 text-muted-foreground">
                If the browser does not return here automatically, copy the
                one-time code shown on the Electric Sheep page. It works only
                for this sign-in attempt.
              </p>
            </form>
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

        <p className="mt-6 text-xs leading-5 text-muted-foreground">
          Provider setup and agent permissions remain inside Hermes. This screen
          verifies only your Electric Sheep membership and managed relay
          identity.
        </p>
      </section>
    </main>
  );
}
