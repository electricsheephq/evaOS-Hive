import assert from "node:assert/strict";
import test from "node:test";

import {
  companyMemberAsSearchResult,
  companyMemberPubkeys,
  isManagedDirectoryCandidate,
  mergeCompanyDirectorySearchResults,
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
