import { invoke, isTauri } from "@tauri-apps/api/core";

export type DesktopProductPolicy = {
  managed: boolean;
  productName: string;
  version: string;
  bundleIdentifier: string;
  deepLinkScheme: "buzz" | "evaos-teams";
  artifactName: string;
  updateChannel: string;
  updaterEnabled: boolean;
  upstreamHostedServicesEnabled: boolean;
  originAttribution: string;
};

export const NATIVE_PRODUCT_POLICY: DesktopProductPolicy = Object.freeze({
  managed: false,
  productName: "Buzz",
  version: "0.4.26",
  bundleIdentifier: "xyz.block.buzz.app",
  deepLinkScheme: "buzz",
  artifactName: "",
  updateChannel: "upstream",
  updaterEnabled: false,
  upstreamHostedServicesEnabled: true,
  originAttribution: "Buzz by Block, licensed under the Apache License 2.0.",
});

let activeProductPolicy = NATIVE_PRODUCT_POLICY;

export function installDesktopProductPolicy(
  policy: DesktopProductPolicy,
): void {
  if (
    !policy.productName ||
    !policy.version ||
    !policy.bundleIdentifier ||
    !["buzz", "evaos-teams"].includes(policy.deepLinkScheme)
  ) {
    throw new Error("Desktop product policy is incomplete");
  }
  activeProductPolicy = Object.freeze({ ...policy });
}

export async function loadDesktopProductPolicy(): Promise<void> {
  if (!isTauri()) return;
  installDesktopProductPolicy(
    await invoke<DesktopProductPolicy>("get_desktop_product_policy"),
  );
}

export function desktopProductPolicy(): DesktopProductPolicy {
  return activeProductPolicy;
}

export function resetDesktopProductPolicyForTests(): void {
  activeProductPolicy = NATIVE_PRODUCT_POLICY;
}
