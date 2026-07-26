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

export type HiveCompanyAgent = {
  agentInstanceId: string;
  publicKey: string;
  displayName: string;
  runtime: string;
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

export function listHiveCompanyAgents() {
  return invoke<HiveCompanyAgent[]>("list_hive_company_agents");
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
    case "active":
      return {
        title: "Access ready",
        body: "Electric Sheep verified this device and selected your Hive community.",
      };
    default:
      return {
        title: "Sign in to Hive",
        body: "Use your Electric Sheep account to connect this device.",
      };
  }
}
