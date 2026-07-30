import { invoke, isTauri } from "@tauri-apps/api/core";

export type DesktopProductPolicy = {
  managed: boolean;
  productName: string;
  version: string;
  bundleIdentifier: string;
  deepLinkScheme: "buzz";
  artifactName: string;
  updateChannel: string;
  updaterEnabled: boolean;
  upstreamHostedServicesEnabled: boolean;
  originAttribution: string;
};

export const NATIVE_PRODUCT_POLICY: DesktopProductPolicy = Object.freeze({
  managed: false,
  productName: "Buzz",
  version: "0.5.2",
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
    policy.deepLinkScheme !== "buzz"
  ) {
    throw new Error("Desktop product policy is incomplete");
  }
  activeProductPolicy = Object.freeze({ ...policy });
}

export async function loadDesktopProductPolicy(): Promise<void> {
  const hasE2eBridge =
    import.meta.env.MODE === "e2e" &&
    Boolean((window as Window & { __BUZZ_E2E__?: unknown }).__BUZZ_E2E__);
  if (!isTauri() && !hasE2eBridge) return;
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
