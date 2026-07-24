import { invoke } from "@tauri-apps/api/core";

export type EvaosTeamsEntitlement = {
  communityId: string;
  relayHost: string;
  publicKey?: string;
  role: string;
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
          "evaOS Teams cannot read its managed identity. Unlock Keychain and try again.",
      };
    case "logout_pending":
      return {
        title: "Finishing sign-out",
        body:
          status.message ??
          "evaOS Teams is disconnected locally and will retry remote revocation.",
      };
    case "reauth_required":
      return {
        title: "Sign in again",
        body:
          status.message ??
          "Your managed access could not be refreshed. evaOS Teams remains disconnected.",
      };
    case "active":
      return {
        title: "Access ready",
        body: "ElectricSheep verified this device and selected your Teams community.",
      };
    default:
      return {
        title: "Sign in to evaOS Teams",
        body: "Use your ElectricSheep account to connect this device.",
      };
  }
}
