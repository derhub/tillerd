import type { NotificationChannelHandle, NotificationWire } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { Store, useSelector } from "@tanstack/react-store";
import { getQueryClient, notificationChannel, query } from "@tillerd/client-bindings";
import React from "react";

import { loadBannerDeps, raiseBanner, type BannerDeps } from "./native-banner";
import { boundedPrepend, countsAsUnread, MAX_ITEMS } from "./store";

interface NotificationsState {
  items: NotificationWire[];
  unread: number;
}

export const notificationsStore = new Store<NotificationsState>({ items: [], unread: 0 });

export function recordNotification(event: NotificationWire): void {
  notificationsStore.setState((s) => ({
    items: boundedPrepend(s.items, event),
    unread: countsAsUnread(event) ? s.unread + 1 : s.unread,
  }));
}

export function startNotifications(
  resolveBanner: () => Promise<BannerDeps | null> = loadBannerDeps,
): () => void {
  let disposed = false;
  let handle: NotificationChannelHandle | undefined;
  void (async () => {
    try {
      const banner = await resolveBanner();
      handle = await notificationChannel((wireEvent) => {
        if (disposed) return;
        recordNotification(wireEvent);
        if (banner) void raiseBanner(wireEvent, banner);
      });
      if (disposed) {
        void handle.close();
        return;
      }
      const history = await getQueryClient().ensureQueryData(query("notificationsList"));
      if (disposed) {
        void handle.close();
        return;
      }
      notificationsStore.setState((s) => {
        const seen = new Set(s.items.map((i) => i.id));
        const merged = [...s.items, ...history.filter((h) => !seen.has(h.id))];
        return { ...s, items: merged.slice(0, MAX_ITEMS) };
      });
    } catch (err) {
      console.error("startNotifications failed:", err);
    }
  })();
  return () => {
    disposed = true;
    void handle?.close();
  };
}

export function markNotificationsRead(): void {
  notificationsStore.setState((s) => ({ ...s, unread: 0 }));
}

// Local mirrors of the server-side disregard mutations -- the feed is a client store hydrated
// once at mount (not a live query subscription), so a successful disregard must also drop the
// row here or it lingers until the next restart's hydration silently omits it.
export function removeNotification(id: string): void {
  notificationsStore.setState((s) => ({ ...s, items: s.items.filter((i) => i.id !== id) }));
}

export function clearNotifications(): void {
  notificationsStore.setState((s) => ({ ...s, items: [] }));
}

export function NotificationsProvider({
  children,
  resolveBanner = loadBannerDeps,
}: {
  children: ReactNode;
  resolveBanner?: () => Promise<BannerDeps | null>;
}) {
  React.useEffect(() => startNotifications(resolveBanner), [resolveBanner]);
  return <>{children}</>;
}

export function useNotifications(): {
  items: NotificationWire[];
  unread: number;
  markRead: () => void;
} {
  const items = useSelector(notificationsStore, (s) => s.items);
  const unread = useSelector(notificationsStore, (s) => s.unread);
  return { items, unread, markRead: markNotificationsRead };
}
