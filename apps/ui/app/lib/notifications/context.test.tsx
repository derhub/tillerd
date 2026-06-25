import type { NotificationWire } from "@tillerd/client-bindings";
import type { ReactNode } from "react";

import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterAll, afterEach, beforeEach, expect, mock, test } from "bun:test";

import { delegatingQuery } from "~/lib/test/real-bindings";

import { NotificationsProvider, notificationsStore, useNotifications } from "./context";

const resetStore = () => notificationsStore.setState(() => ({ items: [], unread: 0 }));
beforeEach(resetStore);
afterEach(() => {
  cleanup();
  resetStore();
});

let notificationHandler: ((event: NotificationWire) => void) | null = null;
let historyData: NotificationWire[] = [];

const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  query: delegatingQuery({ notificationsList: () => ({ queryFn: async () => historyData }) }),
  getQueryClient: () => ({
    ensureQueryData: (opts: { queryFn: () => Promise<NotificationWire[]> }) => opts.queryFn(),
  }),
  notificationChannel: async (cb: (event: NotificationWire) => void) => {
    notificationHandler = cb;
    return {
      close: async () => {
        notificationHandler = null;
      },
    };
  },
}));

afterAll(() => mock.restore());

function ev(id: string): NotificationWire {
  return { id, category: "surface-stopped", severity: "info", message: `m${id}`, ts: Number(id) };
}

function fire(event: NotificationWire): void {
  notificationHandler?.(event);
}

function makeWrapper() {
  const resolveBanner = async () => null;
  return ({ children }: { children: ReactNode }) => (
    <NotificationsProvider resolveBanner={resolveBanner}>{children}</NotificationsProvider>
  );
}

test("hydrates durable history with zero unread", async () => {
  historyData = [ev("2"), ev("1")];
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper() });
  await waitFor(() => expect(result.current.items).toHaveLength(2));
  expect(result.current.items[0].id).toBe("2");
  expect(result.current.unread).toBe(0);
});

test("a live event prepends and increments unread", async () => {
  historyData = [ev("1")];
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper() });
  await waitFor(() => expect(result.current.items).toHaveLength(1));
  act(() => fire(ev("2")));
  await waitFor(() => expect(result.current.items[0].id).toBe("2"));
  expect(result.current.unread).toBe(1);
});

test("markRead clears the unread count", async () => {
  historyData = [];
  const { result } = renderHook(() => useNotifications(), { wrapper: makeWrapper() });
  await waitFor(() => expect(result.current.items).toBeDefined());
  act(() => fire(ev("1")));
  await waitFor(() => expect(result.current.unread).toBe(1));
  act(() => result.current.markRead());
  expect(result.current.unread).toBe(0);
});
