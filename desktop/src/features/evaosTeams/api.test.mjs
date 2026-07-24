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
      assignmentStatus: "unassigned",
      reconciliationStatus: "current",
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

test("pending reconciliation polls quickly without claiming active access", () => {
  const status = {
    managed: true,
    phase: "sync_pending",
    authenticated: true,
    keychainAvailable: true,
    message: "Managed relay projection is still being reconciled",
    entitlement: {
      communityId: "10000000-0000-4000-8000-000000000001",
      relayHost: "https://relay.example.com",
      role: "member",
      assignmentStatus: "unassigned",
      reconciliationStatus: "pending",
      accessRevision: 4,
      expiresAt: "2030-01-01T00:00:00Z",
      refreshAfterSeconds: 300,
    },
  };

  assert.equal(evaosTeamsRefreshDelay(status), 2_000);
  const copy = evaosTeamsStatusCopy(status);
  assert.equal(copy.title, "Finishing managed setup");
  assert.doesNotMatch(JSON.stringify(copy), /access ready|authenticated/i);
});
