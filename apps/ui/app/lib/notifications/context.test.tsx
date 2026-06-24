import type { NotificationWire } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, test } from "bun:test";

import type { NotificationSource } from "~/lib/transport/notification-source";

import { NotificationsProvider, notificationsStore, useNotifications } from "./context";

// Shared module store reset before AND after each test: sibling suite failures that record
// an error notification must not leak into these counts.
const resetStore = () => notificationsStore.setState(() => ({ items: [], unread: 0 }));
beforeEach(resetStore);
afterEach(() => {
  cleanup();
  resetStore();
});

function ev(id: string): NotificationWire {
  return { id, category: "surface-stopped", severity: "info", message: `m${id}`, ts: Number(id) };
}

function fakeSource(history: NotificationWire[]) {
  let emit: ((event: NotificationWire) => void) | null = null;
  const source: NotificationSource = {
    history: async () => history,
    subscribe: async (handler) => {
      emit = handler;
      return () => {};
    },
  };
  return { source, fire: (event: NotificationWire) => emit?.(event) };
}

function makeWrapper(source: NotificationSource) {
  const resolveSource = async () => source;
  const resolveBanner = async () => null;
  return ({ children }: { children: ReactNode }) => (
    <NotificationsProvider resolveSource={resolveSource} resolveBanner={resolveBanner}>
      {children}
    </NotificationsProvider>
  );
}

test("hydrates durable history with zero unread", async () => {
  const { source } = fakeSource([ev("2"), ev("1")]);
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper(source) });
  await waitFor(() => expect(result.current.items).toHaveLength(2));
  expect(result.current.items[0].id).toBe("2");
  expect(result.current.unread).toBe(0);
});

test("a live event prepends and increments unread", async () => {
  const { source, fire } = fakeSource([ev("1")]);
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper(source) });
  await waitFor(() => expect(result.current.items).toHaveLength(1));
  act(() => fire(ev("2")));
  await waitFor(() => expect(result.current.items[0].id).toBe("2"));
  expect(result.current.unread).toBe(1);
});

test("markRead clears the unread count", async () => {
  const { source, fire } = fakeSource([]);
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper(source) });
  await waitFor(() => expect(result.current.items).toBeDefined());
  act(() => fire(ev("1")));
  await waitFor(() => expect(result.current.unread).toBe(1));
  act(() => result.current.markRead());
  expect(result.current.unread).toBe(0);
});
