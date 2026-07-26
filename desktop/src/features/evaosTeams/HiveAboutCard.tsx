import {
  SettingsOptionGroup,
  SettingsOptionRow,
} from "@/features/settings/ui/SettingsOptionGroup";
import { SettingsSectionHeader } from "@/features/settings/ui/SettingsSectionHeader";
import { desktopProductPolicy } from "@/shared/product/productIdentity";

function Value({ children }: { children: string }) {
  return (
    <span className="break-all text-right text-sm text-muted-foreground">
      {children}
    </span>
  );
}

export function HiveAboutCard() {
  const policy = desktopProductPolicy();
  return (
    <section className="min-w-0" data-testid="settings-about">
      <SettingsSectionHeader
        description="Desktop package identity and open-source notices."
        title={`About ${policy.productName}`}
      />
      <SettingsOptionGroup>
        <SettingsOptionRow>
          <span className="text-sm font-medium">Version</span>
          <Value>{policy.version}</Value>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <span className="text-sm font-medium">Bundle ID</span>
          <Value>{policy.bundleIdentifier}</Value>
        </SettingsOptionRow>
        <SettingsOptionRow>
          <span className="text-sm font-medium">Update channel</span>
          <Value>{policy.updateChannel}</Value>
        </SettingsOptionRow>
      </SettingsOptionGroup>
      <div className="mt-4 rounded-xl border border-border/70 bg-muted/20 px-4 py-3 text-sm text-muted-foreground">
        <p>{policy.originAttribution}</p>
        <p className="mt-2">
          The complete Apache-2.0 license and origin notices are included in the
          app bundle.
        </p>
      </div>
    </section>
  );
}
