import type { SettingsSection } from "./ui/SettingsPanels";

export type ManagedSettingsDisposition = "hidden" | "restored" | "native";

const HIDDEN_SECTIONS = new Set<SettingsSection>([
  "compute",
  "hosted-communities",
  "mobile",
]);

const RESTORED_FEATURE_SECTIONS = new Set<SettingsSection>([
  "agents",
  "channel-templates",
  "custom-emoji",
]);

export function managedSettingsDisposition(
  section: SettingsSection,
): ManagedSettingsDisposition {
  if (HIDDEN_SECTIONS.has(section)) return "hidden";
  if (RESTORED_FEATURE_SECTIONS.has(section)) return "restored";
  return "native";
}
