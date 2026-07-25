import assert from "node:assert/strict";
import test from "node:test";

import {
  clearManagedSensitiveRendererState,
  rendererContentPersistenceAllowed,
} from "./managedRendererPersistence.ts";
import {
  installDesktopProductPolicy,
  resetDesktopProductPolicyForTests,
} from "./productIdentity.ts";

const managedPolicy = {
  managed: true,
  productName: "Hive",
  version: "0.4.23-es.1",
  bundleIdentifier: "com.electricsheephq.evaos.teams",
  deepLinkScheme: "evaos-teams",
  artifactName: "Hive-0.4.23-es.1-arm64.dmg",
  updateChannel: "managed-beta",
  updaterEnabled: false,
  upstreamHostedServicesEnabled: false,
  originAttribution:
    "Hive by Electric Sheep. Open-source licenses and origin notices are included with the app.",
};

function storage(entries) {
  const values = new Map(entries);
  return {
    get length() {
      return values.size;
    },
    key: (index) => [...values.keys()][index] ?? null,
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
    removeItem: (key) => values.delete(key),
    clear: () => values.clear(),
  };
}

test("managed bootstrap removes content-bearing records and keeps preferences", () => {
  const target = storage([
    ["buzz-drafts.v1:person", "legacy prompt"],
    ["buzz-drafts.v2:wss://relay:person", "current prompt"],
    ["buzz-channel-messages.v1:wss://relay:room", "relay message"],
    ["buzz-theme", "buzz-dark"],
    ["buzz-read-state.v1:person", "123"],
  ]);

  try {
    installDesktopProductPolicy(managedPolicy);
    assert.equal(rendererContentPersistenceAllowed(), false);
    assert.equal(clearManagedSensitiveRendererState(target), 3);
    assert.equal(target.getItem("buzz-drafts.v1:person"), null);
    assert.equal(
      target.getItem("buzz-channel-messages.v1:wss://relay:room"),
      null,
    );
    assert.equal(target.getItem("buzz-theme"), "buzz-dark");
    assert.equal(target.getItem("buzz-read-state.v1:person"), "123");
  } finally {
    resetDesktopProductPolicyForTests();
  }
});

test("native Buzz keeps renderer persistence unchanged", () => {
  const target = storage([["buzz-drafts.v2:relay:person", "draft"]]);
  resetDesktopProductPolicyForTests();
  assert.equal(rendererContentPersistenceAllowed(), true);
  assert.equal(clearManagedSensitiveRendererState(target), 0);
  assert.equal(target.getItem("buzz-drafts.v2:relay:person"), "draft");
});
