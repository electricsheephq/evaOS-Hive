import assert from "node:assert/strict";
import test from "node:test";

import {
  desktopProductPolicy,
  installDesktopProductPolicy,
  resetDesktopProductPolicyForTests,
} from "./productIdentity.ts";

test("Hive policy keeps its signed updater separate from hosted services", () => {
  installDesktopProductPolicy({
    managed: true,
    productName: "Hive",
    version: "0.5.1-es.1",
    bundleIdentifier: "com.electricsheephq.evaos.teams",
    deepLinkScheme: "buzz",
    artifactName: "Hive-0.5.1-es.1-arm64.dmg",
    updateChannel: "hive-internal",
    updaterEnabled: true,
    upstreamHostedServicesEnabled: false,
    originAttribution: "Built from Buzz by Block.",
  });
  try {
    assert.equal(desktopProductPolicy().productName, "Hive");
    assert.equal(desktopProductPolicy().deepLinkScheme, "buzz");
    assert.equal(desktopProductPolicy().updaterEnabled, true);
    assert.equal(desktopProductPolicy().upstreamHostedServicesEnabled, false);
  } finally {
    resetDesktopProductPolicyForTests();
  }
});
