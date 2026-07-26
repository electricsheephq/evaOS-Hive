import assert from "node:assert/strict";
import test from "node:test";

import {
  companyDirectoryScope,
  companyMemberAsSearchResult,
  companyMemberPubkeys,
  isManagedDirectoryCandidate,
  mergeCompanyDirectorySearchResults,
  retainManagedSelectedRecipients,
} from "./companyMemberDirectory.ts";

const ACTIVE = "a".repeat(64);
const STALE = "b".repeat(64);

test("managed human discovery includes only active company public keys", () => {
  const memberPubkeys = companyMemberPubkeys([
    { publicKey: ACTIVE, displayName: "Andrew" },
  ]);

  assert.equal(
    isManagedDirectoryCandidate({
      candidate: { pubkey: ACTIVE, isAgent: false },
      managed: true,
      memberPubkeys,
    }),
    true,
  );
  assert.equal(
    isManagedDirectoryCandidate({
      candidate: { pubkey: STALE, isAgent: false },
      managed: true,
      memberPubkeys,
    }),
    false,
  );
});

test("managed directory failure stays fail closed for humans", () => {
  assert.equal(
    isManagedDirectoryCandidate({
      candidate: { pubkey: ACTIVE, isAgent: false },
      managed: true,
      memberPubkeys: new Set(),
    }),
    false,
  );
});

test("unmanaged discovery remains relay-native and agents stay separately eligible", () => {
  assert.equal(
    isManagedDirectoryCandidate({
      candidate: { pubkey: STALE, isAgent: false },
      managed: false,
      memberPubkeys: new Set(),
    }),
    true,
  );
  assert.equal(
    isManagedDirectoryCandidate({
      candidate: { pubkey: STALE, isAgent: true },
      managed: true,
      memberPubkeys: new Set(),
    }),
    true,
  );
});

test("company members become public-only native search candidates", () => {
  assert.deepEqual(
    companyMemberAsSearchResult({
      publicKey: ACTIVE,
      displayName: "Andrew",
    }),
    {
      pubkey: ACTIVE,
      displayName: "Andrew",
      avatarUrl: null,
      nip05Handle: null,
      ownerPubkey: null,
      isAgent: false,
    },
  );
});

test("managed directory puts company names first and drops stale humans", () => {
  const results = mergeCompanyDirectorySearchResults({
    managed: true,
    members: [{ publicKey: ACTIVE, displayName: "Andrew" }],
    relayUsers: [
      {
        pubkey: ACTIVE,
        displayName: "stale relay name",
        avatarUrl: "https://relay.invalid/avatar.png",
        nip05Handle: null,
        ownerPubkey: null,
        isAgent: false,
      },
      {
        pubkey: STALE,
        displayName: "obsolete Andrew",
        avatarUrl: null,
        nip05Handle: null,
        ownerPubkey: null,
        isAgent: false,
      },
    ],
  });
  assert.deepEqual(
    results.map(({ pubkey, displayName }) => ({ pubkey, displayName })),
    [
      { pubkey: ACTIVE, displayName: "Andrew" },
      { pubkey: ACTIVE, displayName: "stale relay name" },
    ],
  );
});

test("membership removal prunes a selected human but keeps agents", () => {
  const staleHuman = {
    pubkey: STALE,
    displayName: "Removed member",
    avatarUrl: null,
    nip05Handle: null,
    ownerPubkey: null,
    isAgent: false,
  };
  const agent = {
    ...staleHuman,
    pubkey: "c".repeat(64),
    displayName: "ATRIS",
    isAgent: true,
  };
  assert.deepEqual(
    retainManagedSelectedRecipients({
      managed: true,
      memberPubkeys: new Set([ACTIVE]),
      selected: [staleHuman, agent],
      settled: true,
    }),
    [agent],
  );
});

test("selection is unchanged before settlement and in unmanaged Buzz", () => {
  const selected = [
    {
      pubkey: STALE,
      displayName: "Relay user",
      avatarUrl: null,
      nip05Handle: null,
      ownerPubkey: null,
      isAgent: false,
    },
  ];
  assert.deepEqual(
    retainManagedSelectedRecipients({
      managed: true,
      memberPubkeys: new Set(),
      selected,
      settled: false,
    }),
    selected,
  );
  assert.deepEqual(
    retainManagedSelectedRecipients({
      managed: false,
      memberPubkeys: new Set(),
      selected,
      settled: true,
    }),
    selected,
  );
});

test("managed directory cache scope changes with community or durable identity", () => {
  const status = (communityId, publicKey) => ({
    managed: true,
    phase: "active",
    authenticated: true,
    keychainAvailable: true,
    entitlement: {
      communityId,
      relayHost: "https://relay.example.invalid",
      publicKey,
      role: "member",
      accessRevision: 1,
      expiresAt: "2026-07-27T12:00:00Z",
      refreshAfterSeconds: 300,
    },
  });
  const first = companyDirectoryScope(status("company-a", ACTIVE));
  assert.notEqual(first, companyDirectoryScope(status("company-b", ACTIVE)));
  assert.notEqual(first, companyDirectoryScope(status("company-a", STALE)));
  assert.equal(
    companyDirectoryScope({
      managed: true,
      phase: "reauth_required",
      authenticated: false,
      keychainAvailable: true,
    }),
    null,
  );
});
