import assert from "node:assert/strict";
import test from "node:test";

import {
  filterLocalWelcomeAgentsForPresentation,
  filterLocalWelcomePersonasForPresentation,
  isLocalWelcomeExperienceChannel,
  isLocalWelcomeAgentRecord,
  isLocalWelcomePersonaId,
  shouldPresentLocalAgentTeam,
  shouldPresentLocalWelcomeAgent,
  shouldPresentLocalWelcomePersona,
  shouldRunLocalWelcomeAgent,
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

test("managed Hive does not launch the native local welcome team", () => {
  assert.equal(shouldUseLocalWelcomeTeam(true), false);
});

test("unmanaged Buzz keeps the native local welcome team", () => {
  assert.equal(shouldUseLocalWelcomeTeam(false), true);
});

test("managed Hive skips the native local welcome guide surface", (context) => {
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

test("managed Hive never runs retained local welcome agents", () => {
  assert.equal(
    shouldRunLocalWelcomeAgent(true, {
      personaId: "builtin:fizz",
      teamId: null,
    }),
    false,
  );
  assert.equal(
    shouldRunLocalWelcomeAgent(true, {
      personaId: null,
      teamId: "builtin-team:welcome",
    }),
    false,
  );
  assert.equal(
    shouldRunLocalWelcomeAgent(true, {
      personaId: "custom:researcher",
      teamId: "custom-team:research",
    }),
    true,
  );
  assert.equal(
    shouldRunLocalWelcomeAgent(false, {
      personaId: "builtin:fizz",
      teamId: "builtin-team:welcome",
    }),
    true,
  );
});

test("managed Hive hides only the native built-in local welcome team", () => {
  assert.equal(
    shouldPresentLocalAgentTeam(true, "builtin-team:welcome"),
    false,
  );
  assert.equal(shouldPresentLocalAgentTeam(true, "custom-team:research"), true);
  assert.equal(
    shouldPresentLocalAgentTeam(false, "builtin-team:welcome"),
    true,
  );
});

test("managed presentation hides retained welcome personas and agent records by stable id", () => {
  const personas = [
    { id: "builtin:fizz", displayName: "TARS" },
    { id: "custom:researcher", displayName: "Researcher" },
  ];
  const agents = [
    {
      pubkey: "a".repeat(64),
      personaId: "builtin:honey",
      teamId: null,
    },
    {
      pubkey: "b".repeat(64),
      personaId: "custom:researcher",
      teamId: "custom-team:research",
    },
  ];

  assert.equal(shouldPresentLocalWelcomePersona(true, personas[0]), false);
  assert.equal(shouldPresentLocalWelcomeAgent(true, agents[0]), false);
  assert.deepEqual(
    filterLocalWelcomePersonasForPresentation(true, personas).map(
      (persona) => persona.id,
    ),
    ["custom:researcher"],
  );
  assert.deepEqual(
    filterLocalWelcomeAgentsForPresentation(true, agents).map(
      (agent) => agent.pubkey,
    ),
    ["b".repeat(64)],
  );
});

test("unmanaged Buzz keeps native welcome records and managed Hive keeps custom local records", () => {
  const personas = [{ id: "builtin:bumble" }, { id: "custom:researcher" }];
  const agents = [
    { personaId: "builtin:bumble", teamId: "builtin-team:welcome" },
    { personaId: "custom:researcher", teamId: "custom-team:research" },
  ];

  assert.deepEqual(
    filterLocalWelcomePersonasForPresentation(false, personas),
    personas,
  );
  assert.deepEqual(
    filterLocalWelcomeAgentsForPresentation(false, agents),
    agents,
  );
  assert.equal(shouldPresentLocalWelcomeAgent(true, agents[1]), true);
});
