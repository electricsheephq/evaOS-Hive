import assert from "node:assert/strict";
import test from "node:test";

import {
  createManagedEvaosTeamsAuthority,
  managedAuthorityPolicy,
  managedCommunityFromEntitlement,
} from "./authority.tsx";

const entitlement = {
  communityId: "10000000-0000-4000-8000-000000000001",
  relayHost: "https://relay.example.com",
  publicKey: "ab".repeat(32),
  role: "member",
  assignmentStatus: "unassigned",
  reconciliationStatus: "current",
  accessRevision: 9,
  expiresAt: "2030-01-01T00:00:00Z",
  refreshAfterSeconds: 300,
};

test("managed community is derived only from the server entitlement", () => {
  assert.deepEqual(managedCommunityFromEntitlement(entitlement), {
    id: entitlement.communityId,
    name: "Hive",
    relayUrl: "wss://relay.example.com",
    pubkey: entitlement.publicKey,
    addedAt: entitlement.expiresAt,
  });

  const authority = createManagedEvaosTeamsAuthority(entitlement);
  assert.equal(
    authority.cacheKey,
    `${entitlement.communityId}:${entitlement.publicKey}:9`,
  );
  assert.equal(authority.managed, true);
});

test("managed policy never grants native authority mutations", () => {
  for (const role of ["owner", "admin", "member", "employee", "agent_only"]) {
    const policy = managedAuthorityPolicy(role);
    assert.equal(policy.canManageCommunities, false);
    assert.equal(policy.canManageMembership, false);
    assert.equal(policy.canManageChannels, false);
    assert.equal(policy.canManageAgents, false);
    assert.equal(policy.canBrowseAgents, false);
    assert.equal(policy.canViewMembers, false);
    assert.equal(policy.canStartDirectMessages, false);
  }
});

test("agent-only policy also closes people and private-room discovery", () => {
  const policy = managedAuthorityPolicy("agent_only");
  assert.equal(policy.canBrowsePeople, false);
  assert.equal(policy.canBrowsePrivateRooms, false);
});
