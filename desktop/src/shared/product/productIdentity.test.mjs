import assert from "node:assert/strict";
import test from "node:test";

import {
  desktopProductCopy,
  desktopProductPolicy,
  desktopRuntimePresentation,
  installDesktopProductPolicy,
  resetDesktopProductPolicyForTests,
} from "./productIdentity.ts";

test("Hive policy keeps its signed updater separate from hosted services", () => {
  installDesktopProductPolicy({
    managed: true,
    productName: "Hive",
    version: "0.4.26-es.2",
    bundleIdentifier: "com.electricsheephq.evaos.teams",
    deepLinkScheme: "evaos-teams",
    artifactName: "Hive-0.4.26-es.2-arm64.dmg",
    updateChannel: "hive-internal",
    updaterEnabled: true,
    upstreamHostedServicesEnabled: false,
    originAttribution: "Built from Buzz by Block.",
  });
  try {
    assert.equal(desktopProductPolicy().productName, "Hive");
    assert.equal(desktopProductPolicy().updaterEnabled, true);
    assert.equal(desktopProductPolicy().upstreamHostedServicesEnabled, false);
  } finally {
    resetDesktopProductPolicyForTests();
  }
});

test("managed presentation changes app copy without renaming native identifiers", () => {
  const runtime = {
    id: "buzz-agent",
    label: "Buzz Agent",
    command: "/Applications/Hive.app/Contents/MacOS/buzz-agent",
    defaultArgs: ["--acp"],
    installHint: "Ships with the Buzz desktop app.",
    loginHint: "Buzz requires provider setup.",
  };

  installDesktopProductPolicy({
    managed: true,
    productName: "Hive",
    version: "0.4.26-es.2",
    bundleIdentifier: "com.electricsheephq.evaos.teams",
    deepLinkScheme: "evaos-teams",
    artifactName: "Hive-0.4.26-es.2-arm64.dmg",
    updateChannel: "hive-internal",
    updaterEnabled: true,
    upstreamHostedServicesEnabled: false,
    originAttribution: "Built from Buzz by Block.",
  });
  try {
    assert.equal(
      desktopProductCopy("Save this in your Buzz nest."),
      "Save this in your Hive nest.",
    );
    assert.equal(desktopProductCopy("Buzz Dark"), "Hive Dark");
    assert.equal(
      desktopProductPolicy().originAttribution,
      "Built from Buzz by Block.",
      "managed branding must preserve the configured legal attribution",
    );
    const presented = desktopRuntimePresentation(runtime);
    assert.equal(presented.label, "Hive Agent");
    assert.equal(presented.installHint, "Ships with the Hive desktop app.");
    assert.equal(presented.loginHint, "Hive requires provider setup.");
    assert.equal(presented.id, runtime.id);
    assert.equal(presented.command, runtime.command);
    assert.deepEqual(presented.defaultArgs, runtime.defaultArgs);
  } finally {
    resetDesktopProductPolicyForTests();
  }
});

test("managed presentation productizes runtime hints without renaming vendors", () => {
  const runtime = {
    id: "goose",
    label: "Goose",
    installHint:
      "Buzz requires the Goose CLI; the desktop app alone is not enough.",
    loginHint: null,
  };

  installDesktopProductPolicy({
    managed: true,
    productName: "Hive",
    version: "0.4.26-es.2",
    bundleIdentifier: "com.electricsheephq.evaos.teams",
    deepLinkScheme: "evaos-teams",
    artifactName: "Hive-0.4.26-es.2-arm64.dmg",
    updateChannel: "hive-internal",
    updaterEnabled: true,
    upstreamHostedServicesEnabled: false,
    originAttribution: "Built from Buzz by Block.",
  });
  try {
    const presented = desktopRuntimePresentation(runtime);
    assert.equal(presented.label, "Goose");
    assert.equal(
      presented.installHint,
      "Hive requires the Goose CLI; the desktop app alone is not enough.",
    );
    assert.equal(presented.id, runtime.id);
  } finally {
    resetDesktopProductPolicyForTests();
  }
});

test("native presentation stays Buzz and preserves the runtime object", () => {
  resetDesktopProductPolicyForTests();
  const runtime = {
    id: "buzz-agent",
    label: "Buzz Agent",
    installHint: "Ships with the Buzz desktop app.",
    loginHint: null,
  };
  assert.equal(
    desktopProductCopy("Buzz requires setup."),
    "Buzz requires setup.",
  );
  assert.equal(desktopRuntimePresentation(runtime), runtime);
});
