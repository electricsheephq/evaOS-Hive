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
    | "identity_recovery_required"
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

export type HiveCompanyMember = {
  membershipId: string;
  publicKey: string;
  displayName: string;
};

export type HiveCompanyAgentPolicy = {
  agentInstanceId: string;
  desiredRevision: number;
  appliedRevision: number;
  allowedRoomIds: string[];
  allowedAuthorMembershipIds: string[];
  status: "pending" | "applied" | "error";
  appliedAt: string | null;
  lastErrorCode: string | null;
};

export type SetHiveCompanyAgentPolicyInput = {
  agentInstanceId: string;
  expectedRevision: number;
  allowedRoomIds: string[];
  allowedAuthorMembershipIds: string[];
};

export function getEvaosTeamsAuthStatus() {
  return invoke<EvaosTeamsAuthStatus>("get_evaos_teams_auth_status");
}

export function startEvaosTeamsLogin() {
  return invoke<EvaosTeamsAuthStatus>("start_evaos_teams_login");
}

export function submitEvaosTeamsLoginCode(deviceCode: string) {
  return invoke<void>("submit_evaos_teams_login_code", { deviceCode });
}

export function startEvaosTeamsIdentityRecovery(pairingCode: string) {
  return invoke<void>("start_evaos_teams_identity_recovery", { pairingCode });
}

export function confirmEvaosTeamsIdentityRecoverySas() {
  return invoke<void>("confirm_evaos_teams_identity_recovery_sas");
}

export function cancelEvaosTeamsIdentityRecovery() {
  return invoke<EvaosTeamsAuthStatus>("cancel_evaos_teams_identity_recovery");
}

export function replaceLostEvaosTeamsIdentity() {
  return invoke<EvaosTeamsAuthStatus>("replace_lost_evaos_teams_identity");
}

export function logoutEvaosTeams() {
  return invoke<EvaosTeamsAuthStatus>("logout_evaos_teams");
}

export function listHiveCompanyAgents() {
  return invoke<HiveCompanyAgent[]>("list_hive_company_agents");
}

export function listHiveCompanyMembers() {
  return invoke<HiveCompanyMember[]>("list_hive_company_members");
}

export function getHiveCompanyAgentPolicy(agentInstanceId: string) {
  return invoke<HiveCompanyAgentPolicy>("get_hive_company_agent_policy", {
    agentInstanceId,
  });
}

export function setHiveCompanyAgentPolicy(
  input: SetHiveCompanyAgentPolicyInput,
) {
  return invoke<HiveCompanyAgentPolicy>("set_hive_company_agent_policy", {
    input,
  });
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
    case "identity_recovery_required":
      return {
        title: "Recover this Hive identity",
        body:
          status.message ??
          "Electric Sheep verified this account. Recover the existing Hive identity from an already authorized device to continue.",
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
