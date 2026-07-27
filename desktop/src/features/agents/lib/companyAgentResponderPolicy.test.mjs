import assert from "node:assert/strict";
import test from "node:test";

import {
  canManageCompanyAgentPolicy,
  companyAgentRoomOptions,
} from "./companyAgentResponderPolicy.ts";

function channel(input) {
  return {
    channelType: "stream",
    visibility: "open",
    description: "",
    topic: null,
    purpose: null,
    memberCount: 1,
    memberPubkeys: ["a".repeat(64)],
    lastMessageAt: null,
    archivedAt: null,
    participants: [],
    participantPubkeys: [],
    isMember: true,
    ttlSeconds: null,
    ttlDeadline: null,
    ...input,
  };
}

test("company policy rooms are exact non-DM native memberships", () => {
  const rooms = companyAgentRoomOptions(
    [
      channel({ id: "allowed", name: "Allowed" }),
      channel({ id: "dm", name: "DM", channelType: "dm" }),
      channel({ id: "foreign", name: "Foreign", memberPubkeys: [] }),
      channel({ id: "not-joined", name: "Not joined", isMember: false }),
    ],
    "a".repeat(64),
  );
  assert.deepEqual(
    rooms.map((room) => room.id),
    ["allowed"],
  );
});

test("only owner and admin roles expose policy management", () => {
  assert.equal(canManageCompanyAgentPolicy("owner"), true);
  assert.equal(canManageCompanyAgentPolicy("admin"), true);
  assert.equal(canManageCompanyAgentPolicy("member"), false);
  assert.equal(canManageCompanyAgentPolicy(undefined), false);
});
