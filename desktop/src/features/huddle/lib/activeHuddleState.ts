import type { RelayEvent } from "@/shared/api/types";
import {
  KIND_HUDDLE_ENDED,
  KIND_HUDDLE_PARTICIPANT_JOINED,
  KIND_HUDDLE_PARTICIPANT_LEFT,
  KIND_HUDDLE_STARTED,
} from "@/shared/constants/kinds";
import { isHuddleStartStale } from "./huddleCardState";

/** Active huddle surfaced in a parent channel sidebar row. */
export type ActiveHuddleSummary = {
  ephemeralChannelId: string;
  parentChannelId: string;
  participantPubkeys: ReadonlySet<string>;
  startedAt: number;
  lastEventAt: number;
};

type HuddleLifecycleState = {
  ended: boolean;
  ephemeralChannelId: string;
  lastEventAt: number;
  parentChannelId: string;
  participantPubkeys: Set<string>;
  startedAt: number | null;
};

function tagValue(event: RelayEvent, name: string): string | null {
  return (
    event.tags.find(
      (tag) => tag[0] === name && typeof tag[1] === "string",
    )?.[1] ?? null
  );
}

/** Read the parent channel id from a huddle lifecycle event. */
export function huddleParentChannelId(event: RelayEvent): string | null {
  return tagValue(event, "h");
}

/** Read the ephemeral huddle channel id from a huddle lifecycle payload. */
export function huddleEphemeralChannelId(event: RelayEvent): string | null {
  try {
    const parsed = JSON.parse(event.content) as {
      ephemeral_channel_id?: unknown;
    };
    return typeof parsed.ephemeral_channel_id === "string"
      ? parsed.ephemeral_channel_id
      : null;
  } catch {
    return null;
  }
}

/** Read the participant pubkey affected by a join/leave lifecycle event. */
export function huddleLifecycleParticipant(event: RelayEvent): string | null {
  return tagValue(event, "p") ?? event.pubkey ?? null;
}

function stateKey(parentChannelId: string, ephemeralChannelId: string): string {
  return `${parentChannelId}:${ephemeralChannelId}`;
}

function eventOrder(kind: number): number {
  switch (kind) {
    case KIND_HUDDLE_STARTED:
      return 0;
    case KIND_HUDDLE_PARTICIPANT_JOINED:
      return 1;
    case KIND_HUDDLE_PARTICIPANT_LEFT:
      return 2;
    case KIND_HUDDLE_ENDED:
      return 3;
    default:
      return 4;
  }
}

/** Replays lifecycle rows into one non-stale, non-ended active huddle per parent channel. */
export function reconstructActiveHuddlesByParentChannel(
  events: Iterable<RelayEvent>,
  nowMs = Date.now(),
): Map<string, ActiveHuddleSummary> {
  const states = new Map<string, HuddleLifecycleState>();
  const sorted = [...events].sort(
    (left, right) =>
      left.created_at - right.created_at ||
      eventOrder(left.kind) - eventOrder(right.kind) ||
      left.id.localeCompare(right.id),
  );

  for (const event of sorted) {
    const parentChannelId = huddleParentChannelId(event);
    const ephemeralChannelId = huddleEphemeralChannelId(event);
    if (!parentChannelId || !ephemeralChannelId) continue;

    const key = stateKey(parentChannelId, ephemeralChannelId);
    let state = states.get(key);

    if (!state) {
      state = {
        ended: false,
        ephemeralChannelId,
        lastEventAt: event.created_at,
        parentChannelId,
        participantPubkeys: new Set(),
        startedAt: null,
      };
      states.set(key, state);
    }

    state.lastEventAt = Math.max(state.lastEventAt, event.created_at);

    switch (event.kind) {
      case KIND_HUDDLE_STARTED:
        state.ended = false;
        state.startedAt = event.created_at;
        if (event.pubkey) state.participantPubkeys.add(event.pubkey);
        break;
      case KIND_HUDDLE_PARTICIPANT_JOINED: {
        if (state.ended) break;
        const participant = huddleLifecycleParticipant(event);
        if (participant) state.participantPubkeys.add(participant);
        break;
      }
      case KIND_HUDDLE_PARTICIPANT_LEFT: {
        if (state.ended) break;
        const participant = huddleLifecycleParticipant(event);
        if (participant) state.participantPubkeys.delete(participant);
        break;
      }
      case KIND_HUDDLE_ENDED:
        state.ended = true;
        break;
    }
  }

  const activeByParent = new Map<string, ActiveHuddleSummary>();
  for (const state of states.values()) {
    if (state.ended) continue;
    if (state.startedAt === null) continue;

    if (isHuddleStartStale(state.startedAt, nowMs)) continue;

    const existing = activeByParent.get(state.parentChannelId);
    if (existing && existing.lastEventAt >= state.lastEventAt) continue;

    activeByParent.set(state.parentChannelId, {
      ephemeralChannelId: state.ephemeralChannelId,
      parentChannelId: state.parentChannelId,
      participantPubkeys: state.participantPubkeys,
      startedAt: state.startedAt,
      lastEventAt: state.lastEventAt,
    });
  }

  return activeByParent;
}
