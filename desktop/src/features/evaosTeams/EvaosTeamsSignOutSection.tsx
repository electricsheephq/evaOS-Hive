import * as React from "react";
import { toast } from "sonner";

import { logoutEvaosTeams } from "./api";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/shared/ui/alert-dialog";
import { Button } from "@/shared/ui/button";
import { Spinner } from "@/shared/ui/spinner";

export function EvaosTeamsSignOutSection() {
  const [open, setOpen] = React.useState(false);
  const [pending, setPending] = React.useState(false);

  async function signOut() {
    setPending(true);
    try {
      await logoutEvaosTeams();
      window.localStorage.clear();
      window.sessionStorage.clear();
      window.location.reload();
    } catch (error) {
      setPending(false);
      toast.error(
        error instanceof Error ? error.message : "Managed sign-out failed.",
      );
    }
  }

  return (
    <div
      className="mt-8 border-t border-border/60 pb-6 pt-5"
      data-testid="settings-managed-signout"
    >
      <div className="flex items-center justify-between gap-4 px-1">
        <div className="min-w-0 space-y-1">
          <h2 className="text-lg font-semibold tracking-tight">Sign out</h2>
          <p className="text-sm text-muted-foreground">
            Revokes this managed session and removes its identity from this
            device. Private keys are never shown in Hive.
          </p>
        </div>
        <Button
          className="shrink-0"
          data-testid="managed-signout-open-dialog"
          disabled={pending}
          onClick={() => setOpen(true)}
          type="button"
          variant="destructive"
        >
          {pending ? (
            <Spinner aria-label="Signing out" className="h-4 w-4 border-2" />
          ) : null}
          {pending ? "Signing out…" : "Sign Out"}
        </Button>
      </div>

      <AlertDialog onOpenChange={setOpen} open={open}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Sign out of Hive?</AlertDialogTitle>
            <AlertDialogDescription>
              This device will disconnect from the managed relay and must be
              verified again before it can reconnect.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={pending}>Cancel</AlertDialogCancel>
            <Button
              data-testid="managed-signout-confirm"
              disabled={pending}
              onClick={() => void signOut()}
              variant="destructive"
            >
              Sign Out
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
