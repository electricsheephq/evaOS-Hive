import assert from "node:assert/strict";
import test from "node:test";

import {
  forEachRetiredManagedCommunity,
  isManagedCommunityStateReady,
  managedRelayWebSocketUrl,
  resolveManagedCommunityState,
} from "./managedCommunity.ts";

const ENTITLEMENT = {
  communityId: "10000000-0000-4000-8000-000000000003",
  relayHost: "https://teams.example.invalid",
  publicKey: "a".repeat(64),
  role: "owner",
  accessRevision: 5,
  expiresAt: "2030-01-01T00:00:00Z",
  refreshAfterSeconds: 300,
};

test("managed relay conversion accepts only a credential-free HTTPS origin", () => {
  assert.equal(
    managedRelayWebSocketUrl("https://teams.example.invalid"),
    "wss://teams.example.invalid",
  );
  for (const invalid of [
    "http://teams.example.invalid",
    "https://user@teams.example.invalid",
    "https://teams.example.invalid/path",
    "https://teams.example.invalid?tenant=client",
  ]) {
    assert.throws(() => managedRelayWebSocketUrl(invalid), /HTTPS origin/);
  }
});

test("first managed entitlement creates and selects one native community", () => {
  const state = resolveManagedCommunityState(
    [],
    ENTITLEMENT,
    "2026-07-26T00:00:00.000Z",
  );
  assert.deepEqual(state.communities, [
    {
      id: ENTITLEMENT.communityId,
      name: "Hive",
      relayUrl: "wss://teams.example.invalid",
      pubkey: ENTITLEMENT.publicKey,
      addedAt: "2026-07-26T00:00:00.000Z",
    },
  ]);
  assert.equal(state.activeId, ENTITLEMENT.communityId);
  assert.deepEqual(state.retiredCommunities, []);
  assert.equal(
    isManagedCommunityStateReady(
      state.communities,
      state.activeId,
      ENTITLEMENT,
    ),
    true,
  );
});

test("same company keeps local name and repo configuration", () => {
  const existing = {
    id: ENTITLEMENT.communityId,
    name: "Electric Sheep",
    relayUrl: "wss://old.example.invalid",
    pubkey: "b".repeat(64),
    token: "stale-invite",
    reposDir: "/tmp/repos",
    addedAt: "2026-07-25T00:00:00.000Z",
  };
  const state = resolveManagedCommunityState([existing], ENTITLEMENT);
  assert.deepEqual(state.communities, [
    {
      id: ENTITLEMENT.communityId,
      name: "Electric Sheep",
      relayUrl: "wss://teams.example.invalid",
      pubkey: ENTITLEMENT.publicKey,
      reposDir: "/tmp/repos",
      addedAt: "2026-07-25T00:00:00.000Z",
    },
  ]);
  assert.deepEqual(state.retiredCommunities, [existing]);
});

test("different memberships sharing one relay never reuse local community identity", () => {
  const previous = {
    id: "20000000-0000-4000-8000-000000000004",
    name: "Previous company",
    relayUrl: "wss://teams.example.invalid",
    pubkey: "b".repeat(64),
    addedAt: "2026-07-25T00:00:00.000Z",
  };
  const state = resolveManagedCommunityState(
    [previous],
    ENTITLEMENT,
    "2026-07-26T00:00:00.000Z",
  );
  assert.deepEqual(state.communities, [
    {
      id: ENTITLEMENT.communityId,
      name: "Hive",
      relayUrl: "wss://teams.example.invalid",
      pubkey: ENTITLEMENT.publicKey,
      addedAt: "2026-07-26T00:00:00.000Z",
    },
  ]);
  assert.deepEqual(state.retiredCommunities, [previous]);
  const clearedScopes = [];
  forEachRetiredManagedCommunity(state.retiredCommunities, (community) => {
    clearedScopes.push({
      id: community.id,
      relayUrl: community.relayUrl,
    });
  });
  assert.deepEqual(clearedScopes, [
    {
      id: previous.id,
      relayUrl: previous.relayUrl,
    },
  ]);
  assert.equal(
    isManagedCommunityStateReady(state.communities, previous.id, ENTITLEMENT),
    false,
  );
});

test("unrelated communities on other relays remain available", () => {
  const unrelated = {
    id: "30000000-0000-4000-8000-000000000005",
    name: "Unrelated community",
    relayUrl: "wss://other.example.invalid",
    pubkey: "c".repeat(64),
    addedAt: "2026-07-25T00:00:00.000Z",
  };
  const state = resolveManagedCommunityState([unrelated], ENTITLEMENT);
  assert.equal(state.communities.length, 2);
  assert.equal(state.communities[0].id, unrelated.id);
  assert.equal(state.communities[1].id, ENTITLEMENT.communityId);
  assert.deepEqual(state.retiredCommunities, []);
});

test("persisted same-relay membership conflict forces one cleanup reconciliation", () => {
  const previous = {
    id: "20000000-0000-4000-8000-000000000004",
    name: "Previous company",
    relayUrl: "wss://teams.example.invalid",
    pubkey: "b".repeat(64),
    addedAt: "2026-07-25T00:00:00.000Z",
  };
  const entitled = {
    id: ENTITLEMENT.communityId,
    name: "Hive",
    relayUrl: "wss://teams.example.invalid",
    pubkey: ENTITLEMENT.publicKey,
    addedAt: "2026-07-26T00:00:00.000Z",
  };
  assert.equal(
    isManagedCommunityStateReady(
      [previous, entitled],
      ENTITLEMENT.communityId,
      ENTITLEMENT,
    ),
    false,
  );

  const state = resolveManagedCommunityState([previous, entitled], ENTITLEMENT);
  assert.deepEqual(state.communities, [entitled]);
  assert.deepEqual(state.retiredCommunities, [previous]);
  assert.equal(
    isManagedCommunityStateReady(
      state.communities,
      state.activeId,
      ENTITLEMENT,
    ),
    true,
  );
});

test("persisted active selection must match the entitlement", () => {
  const state = resolveManagedCommunityState([], ENTITLEMENT);
  assert.equal(
    isManagedCommunityStateReady(state.communities, null, ENTITLEMENT),
    false,
  );
  assert.equal(
    isManagedCommunityStateReady(
      state.communities,
      "20000000-0000-4000-8000-000000000004",
      ENTITLEMENT,
    ),
    false,
  );
});
