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

/**
 * Replace the upstream application name in explicitly selected user-facing
 * copy. Callers must not use this for protocol names, binary names, runtime
 * IDs, storage paths, or legal attribution.
 */
export function desktopProductCopy(copy: string): string {
  const policy = desktopProductPolicy();
  return policy.managed ? copy.replace(/\bBuzz\b/g, policy.productName) : copy;
}

type RuntimePresentation = {
  id: string;
  label: string;
  installHint: string;
  loginHint: string | null;
};

/**
 * Productize the built-in desktop agent runtime at the UI boundary while
 * preserving its native runtime ID, command, arguments, and capabilities.
 */
export function desktopRuntimePresentation<T extends RuntimePresentation>(
  runtime: T,
): T {
  const policy = desktopProductPolicy();
  if (!policy.managed || runtime.id !== "buzz-agent") return runtime;
  return {
    ...runtime,
    label: `${policy.productName} Agent`,
    installHint: desktopProductCopy(runtime.installHint),
    loginHint:
      runtime.loginHint === null ? null : desktopProductCopy(runtime.loginHint),
  };
}

export function resetDesktopProductPolicyForTests(): void {
  activeProductPolicy = NATIVE_PRODUCT_POLICY;
}
