import assert from "node:assert/strict";
import test from "node:test";

import {
  NATIVE_PRODUCT_POLICY,
  desktopProductPolicy,
  installDesktopProductPolicy,
  resetDesktopProductPolicyForTests,
} from "./productIdentity.ts";

const managedPolicy = {
  managed: true,
  productName: "evaOS Teams",
  version: "0.4.23-es.1",
  bundleIdentifier: "com.electricsheephq.evaos.teams",
  deepLinkScheme: "evaos-teams",
  artifactName: "evaOS-Teams-0.4.23-es.1-arm64.dmg",
  updateChannel: "managed-beta",
  updaterEnabled: false,
  upstreamHostedServicesEnabled: false,
  originAttribution:
    "Built from Buzz by Block, used under the Apache License 2.0.",
};

test("managed product policy installs the exact package identity", () => {
  try {
    installDesktopProductPolicy(managedPolicy);
    assert.deepEqual(desktopProductPolicy(), managedPolicy);
  } finally {
    resetDesktopProductPolicyForTests();
  }
});

test("managed product policy rejects updater or hosted-service reachability", () => {
  for (const unsafePolicy of [
    { ...managedPolicy, updaterEnabled: true },
    { ...managedPolicy, upstreamHostedServicesEnabled: true },
  ]) {
    assert.throws(
      () => installDesktopProductPolicy(unsafePolicy),
      /not fail-closed/,
    );
  }
  assert.deepEqual(desktopProductPolicy(), NATIVE_PRODUCT_POLICY);
});
