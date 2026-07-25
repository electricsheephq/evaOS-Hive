import assert from "node:assert/strict";
import test from "node:test";

import { managedAuthorityPolicy } from "@/features/evaosTeams/authority";
import { canShowCommunityMembersSettings } from "./settingsNavigation.ts";

function managedAuthority(role) {
  return {
    managed: true,
    cacheKey: `managed:${role}`,
    policy: managedAuthorityPolicy(role),
  };
}

test("managed owners and admins can navigate to workspace invitations", () => {
  for (const role of ["owner", "admin"]) {
    assert.equal(
      canShowCommunityMembersSettings(managedAuthority(role), undefined),
      true,
    );
  }
});

test("managed members cannot navigate to workspace invitations", () => {
  for (const role of ["member", "employee", "agent_only"]) {
    assert.equal(
      canShowCommunityMembersSettings(managedAuthority(role), undefined),
      false,
    );
  }
});

test("native invitation navigation still follows relay membership", () => {
  const nativeAuthority = {
    managed: false,
    cacheKey: "native",
    policy: managedAuthorityPolicy("owner"),
  };
  const lookup = (role) => ({
    snapshotFound: true,
    membershipRequired: true,
    membership: {
      pubkey: "ab".repeat(32),
      role,
      addedBy: null,
      createdAt: "2030-01-01T00:00:00Z",
    },
  });

  assert.equal(
    canShowCommunityMembersSettings(nativeAuthority, lookup("owner")),
    true,
  );
  assert.equal(
    canShowCommunityMembersSettings(nativeAuthority, lookup("member")),
    false,
  );
});
