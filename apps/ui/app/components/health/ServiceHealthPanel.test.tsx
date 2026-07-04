import type { ServiceHealthWire } from "@tillerd/client-bindings";

import {
  createRouter,
  createRootRoute,
  createMemoryHistory,
  RouterProvider,
} from "@tanstack/react-router";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
/// <reference lib="dom" />
import { afterAll, afterEach, expect, mock, test } from "bun:test";
import React from "react";

import type { OrchestratorPhase } from "~/lib/health/aggregate";

import { ServiceHealthPanel } from "./ServiceHealthPanel";

afterEach(cleanup);

// Spies on the health panel's per-service logs control instead of the real settings-store
// side effects it triggers -- this suite only needs to prove the button asks for the right
// service, not that the workbench settings store actually flips.
let showBottomPanelTabCalls: Array<{ tab: string; opts?: { logsService?: string } }> = [];
void mock.module("~/lib/workbench", () => ({
  showBottomPanelTab: (tab: string, opts?: { logsService?: string }) => {
    showBottomPanelTabCalls.push({ tab, opts });
  },
}));
afterAll(() => mock.restore());
afterEach(() => {
  showBottomPanelTabCalls = [];
});

function renderPanel(
  services: ServiceHealthWire[],
  phase: OrchestratorPhase = "ready",
  reason?: string,
) {
  const root = createRootRoute({
    component: () => <ServiceHealthPanel phase={phase} reason={reason} services={services} />,
  });
  const router = createRouter({
    routeTree: root,
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  return render(<RouterProvider router={router} />);
}

test("lists a row for the orchestrator and each service", async () => {
  renderPanel([
    { name: "tillerd-gate", version: "1.0.0", state: "ready" },
    { name: "tillerd-daemon", version: "1.0.0", state: "ready" },
  ]);
  await waitFor(() => expect(screen.queryByText("orchestrator")).not.toBeNull());
  expect(screen.getByText("gate")).toBeTruthy();
  expect(screen.getByText("daemon")).toBeTruthy();
});

test("a service row shows its version and state", async () => {
  renderPanel([{ name: "tillerd-gate", version: "1.2.3", state: "ready" }]);
  await waitFor(() => expect(screen.queryByText("1.2.3")).not.toBeNull());
  expect(screen.getAllByText("ready").length).toBeGreaterThanOrEqual(1);
});

test("a service row's logs control opens the bottom panel's Logs tab filtered to that service", async () => {
  renderPanel([{ name: "tillerd-gate", version: "1.0.0", state: "ready" }]);
  const gateLogs = await screen.findByRole("button", { name: "Show logs for gate" });
  fireEvent.click(gateLogs);
  expect(showBottomPanelTabCalls).toContainEqual({
    tab: "logs",
    opts: { logsService: "tillerd-gate" },
  });

  const orchestratorLogs = screen.getByRole("button", { name: "Show logs for orchestrator" });
  fireEvent.click(orchestratorLogs);
  expect(showBottomPanelTabCalls).toContainEqual({
    tab: "logs",
    opts: { logsService: "tillerd-desktop" },
  });
});

test("a version-mismatched service shows the mismatch state inline", async () => {
  renderPanel([{ name: "tillerd-daemon", version: "0.9.0", state: "versionMismatch" }]);
  await waitFor(() => expect(screen.queryByText("version mismatch")).not.toBeNull());
});

test("the panel exposes no control that changes a service's lifecycle", async () => {
  renderPanel([{ name: "tillerd-gate", version: "1.0.0", state: "ready" }]);
  await waitFor(() => expect(screen.queryByText("gate")).not.toBeNull());
  // Every button in the panel is a "show logs" jump, never a start/stop/restart control.
  const buttons = screen.getAllByRole("button");
  expect(buttons.length).toBeGreaterThan(0);
  expect(buttons.every((b) => (b.getAttribute("aria-label") ?? "").startsWith("Show logs for"))).toBe(
    true,
  );
});

test("the orchestrator row shows its failure reason when the orchestrator failed", async () => {
  renderPanel([], "error", "store open failed");
  await waitFor(() => expect(screen.queryByText("store open failed")).not.toBeNull());
});
