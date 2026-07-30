import assert from "node:assert/strict";
import test from "node:test";

import {
  evaosTeamsGateBypassed,
  evaosTeamsNeedsNativeIdentityRecovery,
  evaosTeamsRefreshDelay,
  evaosTeamsStatusCopy,
} from "./api.ts";

const status = (phase, refreshAfterSeconds) => ({
  managed: true,
  phase,
  authenticated: phase === "active",
  keychainAvailable: true,
  ...(refreshAfterSeconds
    ? {
        entitlement: {
          communityId: "10000000-0000-4000-8000-000000000003",
          relayHost: "https://relay.example.com",
          publicKey: "a".repeat(64),
          role: "member",
          accessRevision: 1,
          expiresAt: "2030-01-01T00:00:00Z",
          refreshAfterSeconds,
        },
      }
    : {}),
});

test("refresh delay is bounded by the validated entitlement interval", () => {
  assert.equal(evaosTeamsRefreshDelay(status("active", 29)), 30_000);
  assert.equal(evaosTeamsRefreshDelay(status("active", 300)), 300_000);
  assert.equal(evaosTeamsRefreshDelay(status("active", 3601)), 3_600_000);
});

test("status copy distinguishes identity restoration from reauthentication", () => {
  assert.match(
    evaosTeamsStatusCopy(status("identity_restore_required")).title,
    /Restore/,
  );
  assert.match(
    evaosTeamsStatusCopy(status("reauth_required")).title,
    /Sign in/,
  );
});

test("unmanaged and native status bypass the managed auth gate", () => {
  assert.equal(evaosTeamsGateBypassed(false, null), true);
  assert.equal(
    evaosTeamsGateBypassed(true, {
      managed: false,
      phase: "native",
      authenticated: false,
      keychainAvailable: true,
    }),
    true,
  );
  assert.equal(evaosTeamsGateBypassed(true, status("signed_out")), false);
});

test("managed canonical-key mismatch hands control to native recovery", () => {
  assert.equal(
    evaosTeamsNeedsNativeIdentityRecovery(status("identity_restore_required")),
    true,
  );
  assert.equal(
    evaosTeamsNeedsNativeIdentityRecovery(status("signed_out")),
    false,
  );
  assert.equal(evaosTeamsNeedsNativeIdentityRecovery(null), false);
});
