import { afterEach, expect, test } from "bun:test";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import type { NotificationEvent } from "@tillerd/sdk/orchestrator";

import { NotificationsProvider, useNotifications } from "./context";
import type { NotificationSource } from "~/lib/transport/notification-source";

afterEach(cleanup);

function ev(id: string): NotificationEvent {
  return { id, category: "surface-stopped", severity: "info", message: `m${id}`, ts: Number(id) };
}

function fakeSource(history: NotificationEvent[]) {
  let emit: ((event: NotificationEvent) => void) | null = null;
  const source: NotificationSource = {
    history: async () => history,
    subscribe: async (handler) => {
      emit = handler;
      return () => {};
    },
  };
  return { source, fire: (event: NotificationEvent) => emit?.(event) };
}

function makeWrapper(source: NotificationSource) {
  // Stable resolver refs so the provider effect does not re-run on each render.
  const resolveSource = async () => source;
  const resolveBanner = async () => null;
  return ({ children }: { children: ReactNode }) => (
    <NotificationsProvider resolveSource={resolveSource} resolveBanner={resolveBanner}>
      {children}
    </NotificationsProvider>
  );
}

// Scenario: Opening the center lists recent notifications (hydrated, zero unread)
test("hydrates durable history with zero unread", async () => {
  const { source } = fakeSource([ev("2"), ev("1")]);
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper(source) });
  await waitFor(() => expect(result.current.items).toHaveLength(2));
  expect(result.current.items[0].id).toBe("2");
  expect(result.current.unread).toBe(0);
});

// Scenario: New notification increments the unread count
test("a live event prepends and increments unread", async () => {
  const { source, fire } = fakeSource([ev("1")]);
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper(source) });
  await waitFor(() => expect(result.current.items).toHaveLength(1));
  act(() => fire(ev("2")));
  await waitFor(() => expect(result.current.items[0].id).toBe("2"));
  expect(result.current.unread).toBe(1);
});

// Scenario: Opening the center clears the unread count
test("markRead clears the unread count", async () => {
  const { source, fire } = fakeSource([]);
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper(source) });
  await waitFor(() => expect(result.current.items).toBeDefined());
  act(() => fire(ev("1")));
  await waitFor(() => expect(result.current.unread).toBe(1));
  act(() => result.current.markRead());
  expect(result.current.unread).toBe(0);
});
