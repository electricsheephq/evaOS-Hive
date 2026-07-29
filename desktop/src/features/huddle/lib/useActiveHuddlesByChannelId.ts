import * as React from "react";

import { relayClient } from "@/shared/api/relayClient";
import type { RelayEvent } from "@/shared/api/types";
import type { Channel } from "@/shared/api/types";
import {
  type ActiveHuddleSummary,
  reconstructActiveHuddlesByParentChannel,
} from "./activeHuddleState";
import { HUDDLE_JOINABLE_WINDOW_SECONDS } from "./huddleCardState";

/** Subscribes to visible channel huddle lifecycle rows and returns active summaries by parent channel id. */
export function useActiveHuddlesByChannelId(
  channels: readonly Channel[],
): ReadonlyMap<string, ActiveHuddleSummary> {
  const channelIdsKey = React.useMemo(
    () =>
      [...new Set(channels.map((channel) => channel.id).filter(Boolean))]
        .sort()
        .join("\n"),
    [channels],
  );
  const [activeHuddles, setActiveHuddles] = React.useState<
    Map<string, ActiveHuddleSummary>
  >(() => new Map());

  React.useEffect(() => {
    const channelIds = channelIdsKey ? channelIdsKey.split("\n") : [];
    if (channelIds.length === 0) {
      setActiveHuddles(new Map());
      return;
    }

    let disposed = false;
    let cleanup: (() => void) | null = null;
    let expiryTimeout: ReturnType<typeof window.setTimeout> | null = null;
    const seenEvents = new Map<string, RelayEvent>();

    function clearExpiryTimeout() {
      if (expiryTimeout === null) return;
      window.clearTimeout(expiryTimeout);
      expiryTimeout = null;
    }

    function updateState() {
      if (disposed) return;
      clearExpiryTimeout();
      const next = reconstructActiveHuddlesByParentChannel(seenEvents.values());
      setActiveHuddles(next);

      const nowMs = Date.now();
      let nextExpiryMs = Number.POSITIVE_INFINITY;
      for (const huddle of next.values()) {
        const anchorSeconds = huddle.startedAt ?? huddle.lastEventAt;
        nextExpiryMs = Math.min(
          nextExpiryMs,
          anchorSeconds * 1000 + HUDDLE_JOINABLE_WINDOW_SECONDS * 1000 - nowMs,
        );
      }
      if (Number.isFinite(nextExpiryMs)) {
        expiryTimeout = window.setTimeout(
          updateState,
          Math.max(0, nextExpiryMs) + 1000,
        );
      }
    }

    relayClient
      .subscribeToHuddleEventsForChannels(channelIds, (event) => {
        if (disposed || seenEvents.has(event.id)) return;
        seenEvents.set(event.id, event);
        updateState();
      })
      .then((dispose) => {
        if (disposed) {
          void dispose();
          return;
        }
        cleanup = () => void dispose();
      })
      .catch((error) => {
        console.error(
          "[useActiveHuddlesByChannelId] subscription failed:",
          error,
        );
      });

    return () => {
      disposed = true;
      clearExpiryTimeout();
      cleanup?.();
      setActiveHuddles(new Map());
    };
  }, [channelIdsKey]);

  return activeHuddles;
}
