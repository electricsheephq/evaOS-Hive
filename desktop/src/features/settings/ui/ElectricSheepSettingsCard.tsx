import * as React from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import {
  getEvaosTeamsAuthStatus,
  type EvaosTeamsAuthStatus,
} from "@/features/evaosTeams/api";
import { desktopProductPolicy } from "@/shared/product/productIdentity";
import { Button } from "@/shared/ui/button";
import { SettingsOptionGroup, SettingsOptionRow } from "./SettingsOptionGroup";
import { SettingsSectionHeader } from "./SettingsSectionHeader";

function valueOrUnavailable(value: string | number | undefined): string {
  return value == null || value === "" ? "Unavailable" : String(value);
}

export function ElectricSheepSettingsCard() {
  const [status, setStatus] = React.useState<EvaosTeamsAuthStatus | null>(null);
  const [error, setError] = React.useState<string | null>(null);
  const [refreshing, setRefreshing] = React.useState(false);

  const refresh = React.useCallback(async () => {
    setRefreshing(true);
    try {
      setStatus(await getEvaosTeamsAuthStatus());
      setError(null);
    } catch (refreshError) {
      setError(
        refreshError instanceof Error
          ? refreshError.message
          : String(refreshError),
      );
    } finally {
      setRefreshing(false);
    }
  }, []);

  React.useEffect(() => {
    void refresh();
  }, [refresh]);

  const entitlement = status?.entitlement;
  const policy = desktopProductPolicy();

  return (
    <section className="min-w-0" data-testid="settings-electric-sheep">
      <SettingsSectionHeader
        title="Electric Sheep"
        description="Your company access and the server-selected Hive community for this device."
      />

      <SettingsOptionGroup>
        <SettingsOptionRow>
          <div className="min-w-0">
            <p className="text-sm font-medium">Managed session</p>
            <p className="break-words text-sm text-muted-foreground">
              {error ?? status?.message ?? valueOrUnavailable(status?.phase)}
            </p>
          </div>
          <Button
            disabled={refreshing}
            onClick={() => void refresh()}
            size="sm"
            variant="outline"
          >
            {refreshing ? "Checking…" : "Refresh"}
          </Button>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <div className="min-w-0">
            <p className="text-sm font-medium">Community</p>
            <p className="break-all text-sm text-muted-foreground">
              {valueOrUnavailable(entitlement?.communityId)}
            </p>
          </div>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <div className="min-w-0">
            <p className="text-sm font-medium">
              Relay selected by Electric Sheep
            </p>
            <p className="break-all text-sm text-muted-foreground">
              {valueOrUnavailable(entitlement?.relayHost)}
            </p>
          </div>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <div className="min-w-0">
            <p className="text-sm font-medium">Role and access revision</p>
            <p className="text-sm text-muted-foreground">
              {entitlement
                ? `${entitlement.role} · ${entitlement.accessRevision}`
                : "Unavailable"}
            </p>
          </div>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <div className="min-w-0">
            <p className="text-sm font-medium">Update channel</p>
            <p className="text-sm text-muted-foreground">
              {policy.updateChannel}
            </p>
          </div>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <div className="min-w-0">
            <p className="text-sm font-medium">Support</p>
            <p className="text-sm text-muted-foreground">
              support@electricsheephq.com
            </p>
          </div>
          <Button
            onClick={() =>
              void openUrl(
                "mailto:support@electricsheephq.com?subject=Hive%20support",
              )
            }
            size="sm"
            variant="outline"
          >
            Email support
          </Button>
        </SettingsOptionRow>
      </SettingsOptionGroup>
    </section>
  );
}
