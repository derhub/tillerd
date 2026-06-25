import type { NotificationWire } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { Store, useSelector } from "@tanstack/react-store";
import React from "react";

import { getQueryClient, query, subscribe } from "@tillerd/client-bindings";

import { loadBannerDeps, raiseBanner, type BannerDeps } from "./native-banner";
import { boundedPrepend, MAX_ITEMS } from "./store";

// TanStack Store owns shared client UI state.
// Durable history hydrates on boot, live events prepend, indicator reads selector-scoped slices.
interface NotificationsState {
  items: NotificationWire[];
  unread: number;
}

export const notificationsStore = new Store<NotificationsState>({ items: [], unread: 0 });

export function recordNotification(event: NotificationWire): void {
  notificationsStore.setState((s) => ({
    items: boundedPrepend(s.items, event),
    unread: s.unread + 1,
  }));
}

// Subscribe before hydrating durable history so no event arriving between the two reads is lost.
// Async setup self-guards against resolving after disposal; no unmount race.
export function startNotifications(
  resolveBanner: () => Promise<BannerDeps | null> = loadBannerDeps,
): () => void {
  let disposed = false;
  let unsub: (() => void) | undefined;
  void (async () => {
    const banner = await resolveBanner();
    unsub = await subscribe("notificationEvent").listen((e) => {
      if (disposed) return;
      recordNotification(e.payload);
      if (banner) void raiseBanner(e.payload, banner);
    });
    if (disposed) {
      unsub();
      return;
    }
    const history = await getQueryClient().ensureQueryData(query("notificationsList"));
    if (disposed) {
      unsub();
      return;
    }
    // History is the durable baseline; keep any live events already received above it.
    notificationsStore.setState((s) => {
      const seen = new Set(s.items.map((i) => i.id));
      const merged = [...s.items, ...history.filter((h) => !seen.has(h.id))];
      return { ...s, items: merged.slice(0, MAX_ITEMS) };
    });
  })();
  return () => {
    disposed = true;
    unsub?.();
  };
}

export function markNotificationsRead(): void {
  notificationsStore.setState((s) => ({ ...s, unread: 0 }));
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
