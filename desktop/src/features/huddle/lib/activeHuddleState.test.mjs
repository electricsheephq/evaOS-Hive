import assert from "node:assert/strict";
import test from "node:test";

import {
  KIND_HUDDLE_ENDED,
  KIND_HUDDLE_PARTICIPANT_JOINED,
  KIND_HUDDLE_PARTICIPANT_LEFT,
  KIND_HUDDLE_STARTED,
} from "@/shared/constants/kinds.ts";
import { HUDDLE_JOINABLE_WINDOW_SECONDS } from "./huddleCardState.ts";
import { reconstructActiveHuddlesByParentChannel } from "./activeHuddleState.ts";

const CREATOR = "a".repeat(64);
const BENJI = "b".repeat(64);
const ANDREW = "c".repeat(64);

function huddleEvent({
  id,
  kind,
  parentChannelId = "general",
  ephemeralChannelId = "huddle-1",
  pubkey = CREATOR,
  createdAt,
  participant,
}) {
  const tags = [["h", parentChannelId]];
  if (participant) tags.push(["p", participant]);

  return {
    id,
    pubkey,
    kind,
    created_at: createdAt,
    content: JSON.stringify({ ephemeral_channel_id: ephemeralChannelId }),
    tags,
    sig: "",
  };
}

test("reconstructActiveHuddlesByParentChannel replays lifecycle for a late mounted sidebar", () => {
  const events = [
    huddleEvent({
      id: "start",
      kind: KIND_HUDDLE_STARTED,
      createdAt: 100,
    }),
    huddleEvent({
      id: "join-benji",
      kind: KIND_HUDDLE_PARTICIPANT_JOINED,
      createdAt: 101,
      participant: BENJI,
      pubkey: "relay",
    }),
  ];

  const active = reconstructActiveHuddlesByParentChannel(events, 110_000);
  const huddle = active.get("general");

  assert.equal(active.size, 1);
  assert.equal(huddle?.ephemeralChannelId, "huddle-1");
  assert.equal(huddle?.participantPubkeys.has(CREATOR), true);
  assert.equal(huddle?.participantPubkeys.has(BENJI), true);
});

test("reconstructActiveHuddlesByParentChannel handles out-of-order replay and suppresses ended huddles", () => {
  const events = [
    huddleEvent({
      id: "late-left",
      kind: KIND_HUDDLE_PARTICIPANT_LEFT,
      createdAt: 102,
      participant: BENJI,
      pubkey: "relay",
    }),
    huddleEvent({
      id: "ended",
      kind: KIND_HUDDLE_ENDED,
      createdAt: 103,
    }),
    huddleEvent({
      id: "start",
      kind: KIND_HUDDLE_STARTED,
      createdAt: 100,
    }),
    huddleEvent({
      id: "join-benji",
      kind: KIND_HUDDLE_PARTICIPANT_JOINED,
      createdAt: 101,
      participant: BENJI,
      pubkey: "relay",
    }),
  ];

  const active = reconstructActiveHuddlesByParentChannel(events, 110_000);

  assert.equal(active.size, 0);
});

test("reconstructActiveHuddlesByParentChannel does not resurrect phantom huddles after an end event", () => {
  const events = [
    huddleEvent({
      id: "start",
      kind: KIND_HUDDLE_STARTED,
      createdAt: 100,
    }),
    huddleEvent({
      id: "ended",
      kind: KIND_HUDDLE_ENDED,
      createdAt: 101,
    }),
    huddleEvent({
      id: "late-join",
      kind: KIND_HUDDLE_PARTICIPANT_JOINED,
      createdAt: 102,
      participant: ANDREW,
      pubkey: "relay",
    }),
  ];

  const active = reconstructActiveHuddlesByParentChannel(events, 110_000);

  assert.equal(active.size, 0);
});

test("reconstructActiveHuddlesByParentChannel filters stale huddles by joinable window", () => {
  const startAt = 1_000;
  const nowSeconds = startAt + HUDDLE_JOINABLE_WINDOW_SECONDS + 1;
  const events = [
    huddleEvent({
      id: "start",
      kind: KIND_HUDDLE_STARTED,
      createdAt: startAt,
    }),
  ];

  const active = reconstructActiveHuddlesByParentChannel(
    events,
    nowSeconds * 1000,
  );

  assert.equal(active.size, 0);
});

test("reconstructActiveHuddlesByParentChannel ignores lifecycle rows without a start event", () => {
  const events = [
    huddleEvent({
      id: "join-only",
      kind: KIND_HUDDLE_PARTICIPANT_JOINED,
      createdAt: 3_500,
      participant: BENJI,
      pubkey: "relay",
    }),
  ];

  const active = reconstructActiveHuddlesByParentChannel(events, 3_600_000);

  assert.equal(active.size, 0);
});

test("reconstructActiveHuddlesByParentChannel keeps parent channels isolated", () => {
  const events = [
    huddleEvent({
      id: "general-start",
      kind: KIND_HUDDLE_STARTED,
      createdAt: 100,
      parentChannelId: "general",
      ephemeralChannelId: "huddle-general",
    }),
    huddleEvent({
      id: "private-start",
      kind: KIND_HUDDLE_STARTED,
      createdAt: 101,
      parentChannelId: "private",
      ephemeralChannelId: "huddle-private",
      pubkey: BENJI,
    }),
  ];

  const active = reconstructActiveHuddlesByParentChannel(events, 110_000);

  assert.equal(active.get("general")?.ephemeralChannelId, "huddle-general");
  assert.equal(active.get("private")?.ephemeralChannelId, "huddle-private");
});
