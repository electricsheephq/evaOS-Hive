import assert from "node:assert/strict";
import test from "node:test";

import {
  clearAllDrafts,
  initDraftStore,
  loadDraftEntry,
  persistDraftEntry,
} from "./useDrafts.ts";
import {
  installDesktopProductPolicy,
  resetDesktopProductPolicyForTests,
} from "@/shared/product/productIdentity.ts";

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

function makeLocalStorage() {
  const values = new Map();
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

test("managed drafts remain usable in memory without entering localStorage", () => {
  const localStorage = makeLocalStorage();
  globalThis.window = { localStorage };
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    get: () => globalThis.window.localStorage,
  });

  try {
    installDesktopProductPolicy(managedPolicy);
    clearAllDrafts();
    initDraftStore("managed-person", "wss://relay.example");
    persistDraftEntry(
      "managed-room",
      "managed prompt must stay in memory",
      "managed-room",
      [],
      [],
    );

    assert.equal(
      loadDraftEntry("managed-room")?.content,
      "managed prompt must stay in memory",
    );
    assert.equal(localStorage.length, 0);
  } finally {
    clearAllDrafts();
    resetDesktopProductPolicyForTests();
  }
});
