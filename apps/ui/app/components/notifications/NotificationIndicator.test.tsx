import type { NotificationWire } from "@tillerd/client-bindings";

import {
  createRouter,
  createRootRoute,
  createMemoryHistory,
  RouterProvider,
} from "@tanstack/react-router";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterEach, expect, test } from "bun:test";
import React from "react";

import { NotificationPanel } from "./NotificationIndicator";

afterEach(cleanup);

// <Link> needs a router context. RouterProvider renders async, so tests use waitFor/findBy.
function renderPanel(items: NotificationWire[]) {
  const root = createRootRoute({
    component: () => <NotificationPanel items={items} />,
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
