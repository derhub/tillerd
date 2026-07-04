import type { NotificationWire } from "@tillerd/client-bindings";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  createRouter,
  createRootRoute,
  createMemoryHistory,
  RouterProvider,
} from "@tanstack/react-router";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, beforeEach, expect, mock, test } from "bun:test";
import React from "react";

import { NotificationPanel } from "./NotificationIndicator";

afterEach(cleanup);

// Records every command() invocation instead of hitting the real transport, and stubs the
// server-truth unread count so the header's "N unread" text is deterministic per test. Every
// other export (query() for keys this suite does not care about, etc.) delegates to the real
// module the same way context.test.tsx's mock.module does.
interface CommandCall {
  key: string;
  args: unknown;
}
let commandCalls: CommandCall[] = [];
let unreadCountValue = 0;

const actualBindings = await import("@tillerd/client-bindings");
void mock.module("@tillerd/client-bindings", () => ({
  ...actualBindings,
  query: (key: string, args?: unknown) =>
    key === "notificationCountUnread"
      ? { queryKey: ["notifications", "countUnread"], queryFn: () => Promise.resolve(unreadCountValue) }
      : (actualBindings.query as (k: string, a?: unknown) => unknown)(key, args),
  command: (key: string) => ({
    mutationFn: (args: unknown) => {
      commandCalls.push({ key, args });
      return Promise.resolve(null);
    },
  }),
}));

afterAll(() => mock.restore());

beforeEach(() => {
  commandCalls = [];
  unreadCountValue = 0;
});

function callsFor(key: string): CommandCall[] {
  return commandCalls.filter((c) => c.key === key);
}

// <Link> needs a router context; the mutation/query hooks need a QueryClient. RouterProvider
// renders async, so tests use waitFor/findBy.
function renderPanel(items: NotificationWire[]) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const root = createRootRoute({
    component: () => (
      <QueryClientProvider client={qc}>
        <NotificationPanel items={items} />
      </QueryClientProvider>
    ),
  });
  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(<RouterProvider router={router} />);
}

function ev(over: Partial<NotificationWire> = {}): NotificationWire {
  return {
    id: "1",
    category: "surface-stopped",
    severity: "info",
    message: "a terminal stopped",
    ts: 0,
    ...over,
  };
}

test("shows an empty state when there are no notifications", async () => {
  renderPanel([]);
  await waitFor(() => expect(screen.queryByTestId("notification-empty")).not.toBeNull());
});

test("renders heading, message, and detail", async () => {
  renderPanel([ev({ title: "Terminal stopped", detail: "exit ok" })]);
  await waitFor(() => expect(screen.queryByText("Terminal stopped")).not.toBeNull());
  expect(screen.getByText("a terminal stopped")).toBeTruthy();
  expect(screen.getByText("exit ok")).toBeTruthy();
});

test("falls back to a category label when there is no title", async () => {
  renderPanel([ev({ category: "service-down", message: "gate is unavailable" })]);
  await waitFor(() => expect(screen.queryByText("Service down")).not.toBeNull());
});

test("renders an unrecognised category by its message", async () => {
  renderPanel([ev({ category: "future-thing", message: "future event" })]);
  await waitFor(() => expect(screen.queryByText("future event")).not.toBeNull());
  expect(screen.getByText("Notification")).toBeTruthy();
});

test("a session notification links to that session", async () => {
  renderPanel([ev({ sessionId: "sess-9" })]);
  await waitFor(() =>
    expect(screen.queryByRole("link", { name: "a terminal stopped" })).not.toBeNull(),
  );
  const link = screen.getByRole("link", { name: "a terminal stopped" });
  expect(link.getAttribute("href")).toBe("/session/sess-9");
});

test("the header shows the server-truth unread count, not a client derivation", async () => {
  unreadCountValue = 3;
  renderPanel([ev()]);
  await waitFor(() => expect(screen.queryByText("3 unread")).not.toBeNull());
});

test("a row's mark-read action fires notificationMarkRead for that id", async () => {
  renderPanel([ev({ id: "n1" })]);
  const button = await screen.findByRole("button", { name: /^Mark read:/ });
  fireEvent.click(button);
  await waitFor(() => expect(callsFor("notificationMarkRead")).toHaveLength(1));
  expect(callsFor("notificationMarkRead")[0]?.args).toEqual({ id: "n1" });
});

test("a row's disregard action fires notificationDisregard for that id", async () => {
  renderPanel([ev({ id: "n1" })]);
  const button = await screen.findByRole("button", { name: /^Disregard:/ });
  fireEvent.click(button);
  await waitFor(() => expect(callsFor("notificationDisregard")).toHaveLength(1));
  expect(callsFor("notificationDisregard")[0]?.args).toEqual({ id: "n1" });
});

test("mark-all-read fires notificationMarkAllRead", async () => {
  renderPanel([ev()]);
  const button = await screen.findByRole("button", { name: "Mark all read" });
  fireEvent.click(button);
  await waitFor(() => expect(callsFor("notificationMarkAllRead")).toHaveLength(1));
});

test("disregard-all fires notificationDisregardAll", async () => {
  renderPanel([ev()]);
  const button = await screen.findByRole("button", { name: "Disregard all" });
  fireEvent.click(button);
  await waitFor(() => expect(callsFor("notificationDisregardAll")).toHaveLength(1));
});

test("the snooze picker offers 15m/1h/8h and fires notificationSnooze with the chosen duration", async () => {
  renderPanel([ev({ id: "n1" })]);
  const trigger = await screen.findByRole("button", { name: /^Snooze:/ });
  const before = Date.now();
  fireEvent.click(trigger);

  const option = await screen.findByText("1h");
  fireEvent.click(option);

  await waitFor(() => expect(callsFor("notificationSnooze")).toHaveLength(1));
  const args = callsFor("notificationSnooze")[0]?.args as { id: string; snoozeUntil: number };
  expect(args.id).toBe("n1");
  // Within a generous window of "now + 1h" -- exact equality would couple the test to Date.now().
  expect(args.snoozeUntil).toBeGreaterThanOrEqual(before + 60 * 60_000);
  expect(args.snoozeUntil).toBeLessThan(before + 60 * 60_000 + 10_000);
});
