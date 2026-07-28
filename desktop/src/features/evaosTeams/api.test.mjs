import assert from "node:assert/strict";
import test from "node:test";

import { evaosTeamsRefreshDelay, evaosTeamsStatusCopy } from "./api.ts";

test("managed auth copy never claims authentication while refresh is unknown", () => {
  const status = {
    managed: true,
    phase: "reauth_required",
    authenticated: false,
    keychainAvailable: true,
    message: "Session refresh failed",
  };
  const copy = evaosTeamsStatusCopy(status);
  assert.equal(copy.title, "Sign in again");
  assert.doesNotMatch(
    JSON.stringify(copy),
    /authenticated|desktop_session|nsec/,
  );
});

test("managed identity recovery copy never claims active authentication", () => {
  const status = {
    managed: true,
    phase: "identity_recovery_required",
    authenticated: false,
    keychainAvailable: true,
    message: "Recover the Hive identity key ending in abcdef12.",
  };
  const copy = evaosTeamsStatusCopy(status);
  assert.equal(copy.title, "Recover this Hive identity");
  assert.match(copy.body, /Recover/);
  assert.doesNotMatch(
    JSON.stringify(copy),
    /authenticated|desktop_session|nsec|private key/i,
  );
});

test("managed entitlement controls a bounded refresh delay", () => {
  const status = {
    managed: true,
    phase: "active",
    authenticated: true,
    keychainAvailable: true,
    entitlement: {
      communityId: "10000000-0000-4000-8000-000000000001",
      relayHost: "https://relay.example.com",
      role: "member",
      accessRevision: 4,
      expiresAt: "2030-01-01T00:00:00Z",
      refreshAfterSeconds: 300,
    },
  };
  assert.equal(evaosTeamsRefreshDelay(status), 300_000);
  assert.equal(
    evaosTeamsRefreshDelay({
      ...status,
      entitlement: { ...status.entitlement, refreshAfterSeconds: 1 },
    }),
    30_000,
  );
});
