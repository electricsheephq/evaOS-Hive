import { isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  type FormEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";

import {
  evaosTeamsRefreshDelay,
  evaosTeamsStatusCopy,
  cancelEvaosTeamsIdentityRecovery,
  confirmEvaosTeamsIdentityRecoverySas,
  getEvaosTeamsAuthStatus,
  replaceLostEvaosTeamsIdentity,
  startEvaosTeamsLogin,
  startEvaosTeamsIdentityRecovery,
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
  const [recoveryCode, setRecoveryCode] = useState("");
  const [recoveryStarted, setRecoveryStarted] = useState(false);
  const [recoverySas, setRecoverySas] = useState<string | null>(null);
  const [recoveryWorking, setRecoveryWorking] = useState(false);
  const [lostDeviceConfirmed, setLostDeviceConfirmed] = useState(false);
  const replacingLostIdentity = useRef(false);

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
    if (!tauri || status?.phase !== "identity_recovery_required") return;
    let cancelled = false;
    const unlisteners: (() => void)[] = [];

    listen<{ sas: string }>("evaos-teams-identity-recovery-sas", (event) => {
      if (!cancelled) {
        setRecoverySas(event.payload.sas);
        setRecoveryWorking(false);
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    });

    listen("evaos-teams-identity-recovery-complete", () => {
      if (!cancelled) {
        setRecoveryStarted(false);
        setRecoverySas(null);
        setRecoveryCode("");
        setRecoveryWorking(false);
        void refresh();
      }
    }).then((fn) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    });

    listen<{ message: string }>(
      "evaos-teams-identity-recovery-error",
      (event) => {
        if (!cancelled) {
          setError(event.payload.message);
          setRecoveryStarted(false);
          setRecoverySas(null);
          setRecoveryWorking(false);
        }
      },
    ).then((fn) => {
      if (cancelled) fn();
      else unlisteners.push(fn);
    });

    return () => {
      cancelled = true;
      for (const fn of unlisteners) fn();
    };
  }, [refresh, status?.phase, tauri]);

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

  async function startIdentityRecovery(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!recoveryCode.trim() || recoveryWorking) return;
    setError(null);
    setRecoverySas(null);
    setRecoveryStarted(true);
    setRecoveryWorking(true);
    try {
      await startEvaosTeamsIdentityRecovery(recoveryCode);
    } catch (recoveryError) {
      setError(
        recoveryError instanceof Error
          ? recoveryError.message
          : String(recoveryError),
      );
      setRecoveryStarted(false);
      setRecoveryWorking(false);
    }
  }

  async function confirmIdentityRecovery() {
    if (recoveryWorking) return;
    setRecoveryWorking(true);
    setError(null);
    try {
      await confirmEvaosTeamsIdentityRecoverySas();
    } catch (recoveryError) {
      setError(
        recoveryError instanceof Error
          ? recoveryError.message
          : String(recoveryError),
      );
      setRecoveryWorking(false);
    }
  }

  async function cancelIdentityRecovery() {
    setRecoveryWorking(true);
    setError(null);
    try {
      setStatus(await cancelEvaosTeamsIdentityRecovery());
      setRecoveryStarted(false);
      setRecoverySas(null);
      setRecoveryCode("");
    } catch (recoveryError) {
      setError(
        recoveryError instanceof Error
          ? recoveryError.message
          : String(recoveryError),
      );
    } finally {
      setRecoveryWorking(false);
    }
  }

  async function replaceLostIdentity() {
    if (working || replacingLostIdentity.current) return;
    replacingLostIdentity.current = true;
    try {
      await run(replaceLostEvaosTeamsIdentity);
      setLostDeviceConfirmed(false);
    } finally {
      replacingLostIdentity.current = false;
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
    status?.phase === "signed_out" ||
    status?.phase === "reauth_required" ||
    status?.phase === "identity_recovery_required";

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
          {status?.phase === "identity_recovery_required" ? (
            <form
              className="flex flex-col gap-2"
              onSubmit={startIdentityRecovery}
            >
              <Input
                aria-label="Hive identity pairing code"
                autoCapitalize="none"
                autoComplete="off"
                disabled={recoveryStarted}
                inputMode="text"
                onChange={(event) => setRecoveryCode(event.target.value)}
                placeholder="Paste pairing code from an authorized Hive device"
                spellCheck={false}
                value={recoveryCode}
              />
              <Button
                disabled={
                  !recoveryCode.trim() || recoveryWorking || recoveryStarted
                }
                type="submit"
                variant="outline"
              >
                {recoveryWorking && !recoverySas
                  ? "Waiting for security code…"
                  : recoveryStarted
                    ? "Recovery started"
                    : "Start identity recovery"}
              </Button>
              {recoverySas || recoveryStarted ? (
                <div className="rounded-lg border border-border/70 p-3">
                  {recoverySas ? (
                    <>
                      <p className="text-xs font-medium text-muted-foreground">
                        Verify this code matches the authorized Hive device
                      </p>
                      <p className="mt-2 font-mono text-2xl font-semibold tracking-[0.2em]">
                        {recoverySas.slice(0, 3)} {recoverySas.slice(3)}
                      </p>
                    </>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      Waiting for the authorized Hive device. You can cancel
                      this recovery attempt at any time.
                    </p>
                  )}
                  <div className="mt-3 flex gap-2">
                    <Button
                      disabled={recoveryWorking}
                      onClick={() => void cancelIdentityRecovery()}
                      type="button"
                      variant="outline"
                    >
                      Cancel
                    </Button>
                    {recoverySas ? (
                      <Button
                        disabled={recoveryWorking}
                        onClick={() => void confirmIdentityRecovery()}
                        type="button"
                      >
                        Codes match
                      </Button>
                    ) : null}
                  </div>
                </div>
              ) : null}
              <p className="text-xs leading-5 text-muted-foreground">
                Open Hive on a device that already has this account, use Mobile
                pairing, copy the pairing code, and verify the security code on
                both devices. Hive will import only the exact identity selected
                by Electric Sheep.
              </p>
              {!lostDeviceConfirmed ? (
                <Button
                  disabled={recoveryWorking || recoveryStarted || working}
                  onClick={() => setLostDeviceConfirmed(true)}
                  type="button"
                  variant="ghost"
                >
                  I no longer have an authorized device
                </Button>
              ) : (
                <div className="rounded-lg border border-destructive/40 bg-destructive/10 p-3">
                  <p className="text-sm leading-6 text-foreground" role="alert">
                    This replaces this member&apos;s Hive identity. The old key
                    loses relay access, and offline messages addressed only to
                    that old key may not be recoverable.
                  </p>
                  <div className="mt-3 flex gap-2">
                    <Button
                      autoFocus
                      disabled={working || recoveryWorking || recoveryStarted}
                      onClick={() => setLostDeviceConfirmed(false)}
                      type="button"
                      variant="outline"
                    >
                      Keep existing identity
                    </Button>
                    <Button
                      disabled={working || recoveryWorking || recoveryStarted}
                      onClick={() => void replaceLostIdentity()}
                      type="button"
                      variant="destructive"
                    >
                      {working
                        ? "Replacing identity…"
                        : "Replace identity on this Mac"}
                    </Button>
                  </div>
                </div>
              )}
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
