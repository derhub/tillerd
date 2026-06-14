import { afterEach, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import type { NotificationEvent } from "@tillerd/sdk/orchestrator";

import { NotificationPanel } from "./NotificationIndicator";

afterEach(cleanup);

function ev(over: Partial<NotificationEvent> = {}): NotificationEvent {
  return {
    id: "1",
    category: "surface-stopped",
    severity: "info",
    message: "a terminal stopped",
    ts: 0,
    ...over,
  };
}

function renderPanel(items: NotificationEvent[]) {
  return render(
    <MemoryRouter>
      <NotificationPanel items={items} />
    </MemoryRouter>,
  );
}

// Scenario: Empty feed shows an empty state
test("shows an empty state when there are no notifications", () => {
  renderPanel([]);
  expect(screen.getByTestId("notification-empty")).toBeTruthy();
});

// Scenario: Title, detail, and severity render when present
test("renders heading, message, and detail", () => {
  renderPanel([ev({ title: "Terminal stopped", detail: "exit ok" })]);
  expect(screen.getByText("Terminal stopped")).toBeTruthy();
  expect(screen.getByText("a terminal stopped")).toBeTruthy();
  expect(screen.getByText("exit ok")).toBeTruthy();
});

// Scenario: Missing title falls back to a category label
test("falls back to a category label when there is no title", () => {
  renderPanel([ev({ category: "service-down", message: "gate is unavailable" })]);
  expect(screen.getByText("Service down")).toBeTruthy();
});

// Scenario: An unrecognised category still renders
test("renders an unrecognised category by its message", () => {
  renderPanel([ev({ category: "future-thing", message: "future event" })]);
  expect(screen.getByText("future event")).toBeTruthy();
  expect(screen.getByText("Notification")).toBeTruthy();
});

// Scenario: Activating a session notification navigates in-app (href is correct; e2e asserts outcome)
test("a session notification links to that session", () => {
  renderPanel([ev({ sessionId: "sess-9" })]);
  const link = screen.getByRole("link", { name: "a terminal stopped" });
  expect(link.getAttribute("href")).toBe("/session/sess-9");
});

// Scenario: Actions render as activatable controls
test("renders actions as links to their targets", () => {
  renderPanel([ev({ actions: [{ label: "Open logs", to: "/logs?service=gate" }] })]);
  const link = screen.getByRole("link", { name: "Open logs" });
  expect(link.getAttribute("href")).toBe("/logs?service=gate");
});
