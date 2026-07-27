import assert from "node:assert/strict";
import test from "node:test";

import {
  isLocalWelcomeExperienceChannel,
  isLocalWelcomeAgentRecord,
  isLocalWelcomePersonaId,
  shouldUseLocalWelcomeTeam,
} from "./localWelcomeTeamPolicy.ts";
import {
  installDesktopProductPolicy,
  NATIVE_PRODUCT_POLICY,
  resetDesktopProductPolicyForTests,
} from "../../shared/product/productIdentity.ts";

const welcomeChannel = {
  name: "Welcome",
  channelType: "stream",
  visibility: "private",
};

test("managed Hive skips the local welcome team", () => {
  assert.equal(shouldUseLocalWelcomeTeam(true), false);
});

test("unmanaged Buzz keeps the native local welcome team", () => {
  assert.equal(shouldUseLocalWelcomeTeam(false), true);
});

test("managed Hive suppresses the local welcome guide surface", (context) => {
  context.after(resetDesktopProductPolicyForTests);
  installDesktopProductPolicy({
    ...NATIVE_PRODUCT_POLICY,
    managed: true,
    productName: "Hive",
  });

  assert.equal(isLocalWelcomeExperienceChannel(welcomeChannel), false);
});

test("unmanaged Buzz keeps the native welcome guide surface", (context) => {
  context.after(resetDesktopProductPolicyForTests);

  assert.equal(isLocalWelcomeExperienceChannel(welcomeChannel), true);
});

test("managed filtering follows stable welcome persona ids after a rename", () => {
  assert.equal(isLocalWelcomePersonaId("builtin:fizz"), true);
  assert.equal(isLocalWelcomePersonaId("custom:tars"), false);
});

test("managed filtering recognizes both welcome personas and team records", () => {
  assert.equal(
    isLocalWelcomeAgentRecord({
      personaId: "builtin:honey",
      teamId: null,
    }),
    true,
  );
  assert.equal(
    isLocalWelcomeAgentRecord({
      personaId: null,
      teamId: "builtin-team:welcome",
    }),
    true,
  );
  assert.equal(
    isLocalWelcomeAgentRecord({
      personaId: "custom:researcher",
      teamId: "custom-team:research",
    }),
    false,
  );
});
