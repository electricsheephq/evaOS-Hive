import assert from "node:assert/strict";
import test from "node:test";

import {
  isRawIdentityPlaceholder,
  mergeCompanyAgentSearchResults,
  preferIdentityDisplayName,
  resolveCompanyAgentBatch,
  resolveCompanyAgentPresence,
  resolveCompanyAgentProfile,
} from "./companyAgentIdentity.ts";

const PUBKEY = `${"a".repeat(60)}beef`;
const CATALOG = {
  agentInstanceId: "10000000-0000-4000-8000-000000000001",
  publicKey: PUBKEY,
  displayName: "ATRIS",
  runtime: "hermes",
};

test("native signed display data wins over the company fallback", () => {
  const profile = resolveCompanyAgentProfile(
    {
      pubkey: PUBKEY,
      displayName: "Atris Signed",
      avatarUrl: "https://example.test/atris.png",
      about: null,
      nip05Handle: null,
      ownerPubkey: null,
      hasProfileEvent: true,
    },
    CATALOG,
  );
  assert.equal(profile.displayName, "Atris Signed");
  assert.equal(profile.avatarUrl, "https://example.test/atris.png");
  assert.equal(profile.hasProfileEvent, true);
});

test("company name replaces missing and raw-key profile placeholders", () => {
  assert.equal(isRawIdentityPlaceholder(PUBKEY, PUBKEY), true);
  assert.equal(
    isRawIdentityPlaceholder(
      `${PUBKEY.slice(0, 8)}…${PUBKEY.slice(-4)}`,
      PUBKEY,
    ),
    true,
  );
  assert.equal(
    preferIdentityDisplayName(PUBKEY, CATALOG.displayName, PUBKEY),
    "ATRIS",
  );
  const profile = resolveCompanyAgentProfile(
    {
      pubkey: PUBKEY,
      displayName: null,
      avatarUrl: null,
      about: null,
      nip05Handle: null,
      ownerPubkey: null,
      hasProfileEvent: false,
    },
    CATALOG,
  );
  assert.equal(profile.displayName, "ATRIS");
  assert.equal(profile.hasProfileEvent, false);
});

test("batch fallback adds only requested company agents and preserves signed avatar", () => {
  const response = resolveCompanyAgentBatch(
    {
      profiles: {
        [PUBKEY]: {
          displayName: PUBKEY,
          name: null,
          avatarUrl: "https://example.test/signed.png",
          nip05Handle: null,
          ownerPubkey: null,
          isAgent: true,
        },
      },
      missing: [PUBKEY, "b".repeat(64)],
    },
    [PUBKEY],
    [CATALOG],
  );
  assert.equal(response?.profiles[PUBKEY]?.displayName, "ATRIS");
  assert.equal(
    response?.profiles[PUBKEY]?.avatarUrl,
    "https://example.test/signed.png",
  );
  assert.deepEqual(response?.missing, ["b".repeat(64)]);
});

test("managed search adds catalog-only agents without replacing valid signed names", () => {
  const results = mergeCompanyAgentSearchResults({
    companyAgents: [CATALOG],
    limit: 10,
    query: "atris",
    relayUsers: [
      {
        pubkey: PUBKEY,
        displayName: "Atris Signed",
        avatarUrl: "https://example.test/signed.png",
        nip05Handle: null,
        ownerPubkey: null,
        isAgent: true,
      },
    ],
  });
  assert.equal(results.length, 1);
  assert.equal(results[0].displayName, "Atris Signed");
  assert.equal(results[0].isAgent, true);
});

test("presence is never inferred from registration or away status", () => {
  assert.equal(resolveCompanyAgentPresence(undefined), "unknown");
  assert.equal(resolveCompanyAgentPresence("away"), "unknown");
  assert.equal(resolveCompanyAgentPresence("online"), "online");
  assert.equal(resolveCompanyAgentPresence("offline"), "offline");
});
