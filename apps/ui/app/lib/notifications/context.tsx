import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { NotificationEvent } from "@tillerd/sdk/orchestrator";

import {
  loadNotificationSource,
  type NotificationSource,
} from "~/lib/transport/notification-source";

import { loadBannerDeps, raiseBanner, type BannerDeps } from "./native-banner";
import { boundedPrepend, MAX_ITEMS } from "./store";

interface NotificationsState {
  /** Recent notifications, most recent first. */
  items: NotificationEvent[];
  /** Count recorded since the center was last opened. */
  unread: number;
  /** Clear the unread count (called when the center opens). */
  markRead: () => void;
}

const NotificationsContext = createContext<NotificationsState | null>(null);

/**
 * Single reactive source of truth for the notification center. Hydrates durable history on boot,
 * appends live events, tracks unread since last open, and raises a native banner for background
 * events. `null` source (off the desktop host) degrades to an empty feed. Inject resolvers in tests.
 */
export function NotificationsProvider({
  children,
  resolveSource = loadNotificationSource,
  resolveBanner = loadBannerDeps,
}: {
  children: ReactNode;
  resolveSource?: () => Promise<NotificationSource | null>;
  resolveBanner?: () => Promise<BannerDeps | null>;
}) {
  const [items, setItems] = useState<NotificationEvent[]>([]);
  const [unread, setUnread] = useState(0);

  useEffect(() => {
    let cancelled = false;
    let unsub: (() => void) | undefined;
    void (async () => {
      const source = await resolveSource();
      if (cancelled || !source) return;
      const banner = await resolveBanner();
      // Subscribe before hydrating so no event arriving between the two reads is lost.
      unsub = await source.subscribe((event) => {
        if (cancelled) return;
        setItems((prev) => boundedPrepend(prev, event));
        setUnread((n) => n + 1);
        if (banner) void raiseBanner(event, banner);
      });
      const history = await source.history();
      if (cancelled) {
        unsub?.();
        return;
      }
      // History is the durable baseline; keep any live events already received above it.
      setItems((live) => {
        const seen = new Set(live.map((i) => i.id));
        return [...live, ...history.filter((h) => !seen.has(h.id))].slice(0, MAX_ITEMS);
      });
    })();
    return () => {
      cancelled = true;
      unsub?.();
    };
  }, [resolveSource, resolveBanner]);

  const markRead = useCallback(() => setUnread(0), []);
  const state = useMemo<NotificationsState>(
    () => ({ items, unread, markRead }),
    [items, unread, markRead],
  );
  return <NotificationsContext value={state}>{children}</NotificationsContext>;
}

/** The notification feed as reactive shared state. Empty + no-op off the desktop host. */
export function useNotifications(): NotificationsState {
  return useContext(NotificationsContext) ?? { items: [], unread: 0, markRead: () => {} };
}
