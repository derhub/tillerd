import type { ServiceHealthWire } from "@tillerd/client-bindings";

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

import type { OrchestratorPhase } from "~/lib/health/aggregate";

import { ServiceHealthPanel } from "./ServiceHealthPanel";

afterEach(cleanup);

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

test("a service row links to the logs viewer filtered to that service", async () => {
  renderPanel([{ name: "tillerd-gate", version: "1.0.0", state: "ready" }]);
  await waitFor(() =>
    expect(screen.queryAllByRole("link", { name: /logs/i }).length).toBeGreaterThan(0),
  );
  const hrefs = screen.getAllByRole("link", { name: /logs/i }).map((l) => l.getAttribute("href"));
  expect(hrefs).toContain("/logs?service=tillerd-gate");
  expect(hrefs).toContain("/logs?service=tillerd-desktop");
});

test("a version-mismatched service shows the mismatch state inline", async () => {
  renderPanel([{ name: "tillerd-daemon", version: "0.9.0", state: "versionMismatch" }]);
  await waitFor(() => expect(screen.queryByText("version mismatch")).not.toBeNull());
});

test("the panel exposes no control that changes a service's lifecycle", async () => {
  const { container } = renderPanel([{ name: "tillerd-gate", version: "1.0.0", state: "ready" }]);
  await waitFor(() => expect(screen.queryByText("gate")).not.toBeNull());
  expect(container.querySelectorAll("button").length).toBe(0);
});

test("the orchestrator row shows its failure reason when the orchestrator failed", async () => {
  renderPanel([], "error", "store open failed");
  await waitFor(() => expect(screen.queryByText("store open failed")).not.toBeNull());
});
