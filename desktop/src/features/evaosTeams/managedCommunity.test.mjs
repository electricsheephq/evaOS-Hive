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

test("fresh entitlement creates and selects one native community without token", () => {
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
  assert.equal(state.communities[0].token, undefined);
  assert.equal(
    isManagedCommunityStateReady(
      state.communities,
      state.activeId,
      ENTITLEMENT,
    ),
    true,
  );
});

test("authoritative update preserves local name and repo config but removes token", () => {
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

test("same-relay conflicting membership is retired and its caches are clearable", () => {
  const previous = {
    id: "20000000-0000-4000-8000-000000000004",
    name: "Previous company",
    relayUrl: "wss://teams.example.invalid",
    pubkey: "b".repeat(64),
    addedAt: "2026-07-25T00:00:00.000Z",
  };
  const state = resolveManagedCommunityState([previous], ENTITLEMENT);
  assert.equal(state.communities.length, 1);
  assert.equal(state.communities[0].id, ENTITLEMENT.communityId);
  assert.deepEqual(state.retiredCommunities, [previous]);
  const cleared = [];
  forEachRetiredManagedCommunity(state.retiredCommunities, (community) => {
    cleared.push([community.id, community.relayUrl]);
  });
  assert.deepEqual(cleared, [[previous.id, previous.relayUrl]]);
});

test("unrelated relays are preserved", () => {
  const unrelated = {
    id: "30000000-0000-4000-8000-000000000005",
    name: "Unrelated",
    relayUrl: "wss://other.example.invalid",
    pubkey: "c".repeat(64),
    addedAt: "2026-07-25T00:00:00.000Z",
  };
  const state = resolveManagedCommunityState([unrelated], ENTITLEMENT);
  assert.deepEqual(
    state.communities.map(({ id }) => id),
    [unrelated.id, ENTITLEMENT.communityId],
  );
  assert.deepEqual(state.retiredCommunities, []);
});

test("readiness requires selected exact id relay pubkey and no conflicting row", () => {
  const state = resolveManagedCommunityState([], ENTITLEMENT);
  assert.equal(
    isManagedCommunityStateReady(state.communities, null, ENTITLEMENT),
    false,
  );
  assert.equal(
    isManagedCommunityStateReady(
      [
        ...state.communities,
        {
          ...state.communities[0],
          id: "20000000-0000-4000-8000-000000000004",
          pubkey: "b".repeat(64),
        },
      ],
      state.activeId,
      ENTITLEMENT,
    ),
    false,
  );
});
