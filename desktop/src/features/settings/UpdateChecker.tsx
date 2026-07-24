import { openUrl } from "@tauri-apps/plugin-opener";
import { useUpdaterContext } from "./hooks/UpdaterProvider";
import { Button } from "@/shared/ui/button";
import {
  SettingsOptionGroup,
  SettingsOptionRow,
} from "./ui/SettingsOptionGroup";
import { SettingsSectionHeader } from "./ui/SettingsSectionHeader";
import { desktopProductPolicy } from "@/shared/product/productIdentity";
export function UpdateChecker() {
  const { status, checkForUpdate, installAndRelaunch } = useUpdaterContext();
  const policy = desktopProductPolicy();

  return (
    <section className="min-w-0" data-testid="settings-updates">
      <SettingsSectionHeader
        title="Software Updates"
        description={`Keep ${policy.productName} up to date with the latest features and fixes.`}
      />

      <SettingsOptionGroup>
        {status.state === "managed" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Managed updates</p>
              <p className="text-sm font-normal text-muted-foreground">
                {status.message}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                Channel: {policy.updateChannel}. This app does not contact the
                upstream update feed.
              </p>
            </div>
          </SettingsOptionRow>
        )}
        {status.state === "idle" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Check if a new version is available.
              </p>
            </div>
            <Button size="sm" onClick={checkForUpdate}>
              Check for Updates
            </Button>
          </SettingsOptionRow>
        )}

        {status.state === "checking" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Checking for updates...
              </p>
            </div>
          </SettingsOptionRow>
        )}

        {status.state === "up-to-date" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                You're on the latest version.
              </p>
            </div>
            <Button variant="outline" size="sm" onClick={checkForUpdate}>
              Check Again
            </Button>
          </SettingsOptionRow>
        )}

        {status.state === "unavailable" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Automatic updates aren't available on this build. Download the
                latest release manually.
              </p>
            </div>
            <Button variant="outline" size="sm" onClick={checkForUpdate}>
              Check Again
            </Button>
          </SettingsOptionRow>
        )}

        {status.state === "manual-required" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">
                Update available — v{status.version}
              </p>
              <p className="text-sm font-normal text-muted-foreground">
                In-app updates aren't supported on this Linux package. Download
                the new version from GitHub.{" "}
                <span className="text-muted-foreground">
                  Switch to the AppImage build for automatic updates.
                </span>
              </p>
            </div>
            <Button size="sm" onClick={() => void openUrl(status.releaseUrl)}>
              Download Update
            </Button>
          </SettingsOptionRow>
        )}

        {status.state === "available" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Preparing update...
              </p>
            </div>
          </SettingsOptionRow>
        )}

        {status.state === "downloading" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Downloading update...
              </p>
            </div>
          </SettingsOptionRow>
        )}

        {status.state === "installing" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Installing update...
              </p>
            </div>
          </SettingsOptionRow>
        )}

        {status.state === "ready" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-muted-foreground">
                Update downloaded. Click to apply.
              </p>
            </div>
            <Button size="sm" onClick={installAndRelaunch}>
              Update Now
            </Button>
          </SettingsOptionRow>
        )}

        {status.state === "error" && (
          <SettingsOptionRow>
            <div className="min-w-0">
              <p className="text-sm font-medium">Update status</p>
              <p className="text-sm font-normal text-destructive">
                Update failed: {status.message}
              </p>
            </div>
            <Button variant="outline" size="sm" onClick={checkForUpdate}>
              Retry
            </Button>
          </SettingsOptionRow>
        )}
      </SettingsOptionGroup>
    </section>
  );
}
