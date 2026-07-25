import { invoke } from "@tauri-apps/api/core";

export type EvaosTeamsEntitlement = {
  communityId: string;
  relayHost: string;
  publicKey?: string;
  role: "owner" | "admin" | "member" | "employee" | "agent_only";
  assignmentStatus: "unassigned" | "pending" | "assigned";
  reconciliationStatus: "pending" | "failed" | "current";
  accessRevision: number;
  expiresAt: string;
  refreshAfterSeconds: number;
};

export type EvaosTeamsAuthStatus = {
  managed: boolean;
  phase:
    | "native"
    | "signed_out"
    | "active"
    | "sync_pending"
    | "keychain_locked"
    | "reauth_required"
    | "logout_pending";
  authenticated: boolean;
  keychainAvailable: boolean;
  message?: string;
  entitlement?: EvaosTeamsEntitlement;
};

export function getEvaosTeamsAuthStatus() {
  return invoke<EvaosTeamsAuthStatus>("get_evaos_teams_auth_status");
}

export function startEvaosTeamsLogin() {
  return invoke<EvaosTeamsAuthStatus>("start_evaos_teams_login");
}

export function logoutEvaosTeams() {
  return invoke<EvaosTeamsAuthStatus>("logout_evaos_teams");
}

export function evaosTeamsRefreshDelay(status: EvaosTeamsAuthStatus) {
  if (status.phase === "sync_pending") {
    return status.entitlement?.reconciliationStatus === "failed"
      ? 10_000
      : 2_000;
  }
  const seconds = status.entitlement?.refreshAfterSeconds ?? 300;
  return Math.min(Math.max(seconds, 30), 3600) * 1000;
}

export function evaosTeamsStatusCopy(status: EvaosTeamsAuthStatus) {
  switch (status.phase) {
    case "keychain_locked":
      return {
        title: "Unlock macOS Keychain",
        body:
          status.message ??
          "Hive cannot read its managed identity. Unlock Keychain and try again.",
      };
    case "logout_pending":
      return {
        title: "Finishing sign-out",
        body:
          status.message ??
          "Hive is disconnected locally and will retry remote revocation.",
      };
    case "reauth_required":
      return {
        title: "Sign in again",
        body:
          status.message ??
          "Your managed access could not be refreshed. Hive remains disconnected.",
      };
    case "sync_pending":
      return {
        title: "Finishing managed setup",
        body:
          status.message ??
          "Your identity is verified. Hive is waiting for the managed relay projection.",
      };
    case "active":
      return {
        title: "Access ready",
        body: "ElectricSheep verified this device and selected your Hive workspace.",
      };
    default:
      return {
        title: "Sign in to Hive",
        body: "Use your ElectricSheep account to connect this device.",
      };
  }
}
