import { afterEach, expect, test } from "bun:test";
import { cleanup, render, screen } from "@testing-library/react";
import type { ServiceHealth } from "@tillerd/sdk/orchestrator";

import { ServiceHealthPanel } from "./ServiceHealthPanel";
import type { OrchestratorPhase } from "~/lib/health/aggregate";

afterEach(cleanup);

function renderPanel(
  services: ServiceHealth[],
  phase: OrchestratorPhase = "ready",
  reason?: string,
  onOpenLogs: (service: string) => void = () => {},
) {
  return render(
    <ServiceHealthPanel
      phase={phase}
      reason={reason}
      services={services}
      onOpenLogs={onOpenLogs}
    />,
  );
}

// Scenario: Panel lists each service
test("lists a row for the orchestrator and each service", () => {
  renderPanel([
    { name: "tillerd-gate", version: "1.0.0", state: "ready" },
    { name: "tillerd-daemon", version: "1.0.0", state: "ready" },
  ]);
  expect(screen.getByText("orchestrator")).toBeTruthy();
  expect(screen.getByText("gate")).toBeTruthy();
  expect(screen.getByText("daemon")).toBeTruthy();
});

// Scenario: Row reveals version and state
test("a service row shows its version and state", () => {
  renderPanel([{ name: "tillerd-gate", version: "1.2.3", state: "ready" }]);
  expect(screen.getByText("1.2.3")).toBeTruthy();
  expect(screen.getAllByText("ready").length).toBeGreaterThanOrEqual(1);
});

// Scenario: Row links to that service's logs (href is correct)
test("a service row links to the logs viewer filtered to that service", () => {
  renderPanel([{ name: "tillerd-gate", version: "1.0.0", state: "ready" }]);
  const hrefs = screen.getAllByRole("link", { name: /logs/i }).map((l) => l.getAttribute("href"));
  expect(hrefs).toContain("/logs?service=tillerd-gate");
  expect(hrefs).toContain("/logs?service=tillerd-desktop");
});

// Scenario: Row links to that service's logs (click opens that service, client-side)
test("clicking a row's logs link opens that service's logs without a hard navigation", () => {
  const opened: string[] = [];
  let defaultPrevented = false;
  renderPanel(
    [{ name: "tillerd-gate", version: "1.0.0", state: "ready" }],
    "ready",
    undefined,
    (s) => opened.push(s),
  );
  const gateLink = screen
    .getAllByRole("link", { name: /logs/i })
    .find((l) => l.getAttribute("href") === "/logs?service=tillerd-gate");
  expect(gateLink).toBeTruthy();
  const event = new MouseEvent("click", { bubbles: true, cancelable: true });
  gateLink!.dispatchEvent(event);
  defaultPrevented = event.defaultPrevented;
  expect(opened).toEqual(["tillerd-gate"]);
  expect(defaultPrevented).toBe(true);
});

// Scenario: Version mismatch shown inline
test("a version-mismatched service shows the mismatch state inline", () => {
  renderPanel([{ name: "tillerd-daemon", version: "0.9.0", state: "versionMismatch" }]);
  expect(screen.getByText("version mismatch")).toBeTruthy();
});

// Scenario: No lifecycle controls present
test("the panel exposes no control that changes a service's lifecycle", () => {
  const { container } = renderPanel([{ name: "tillerd-gate", version: "1.0.0", state: "ready" }]);
  expect(container.querySelectorAll("button").length).toBe(0);
});

test("the orchestrator row shows its failure reason when the orchestrator failed", () => {
  renderPanel([], "error", "store open failed");
  expect(screen.getByText("store open failed")).toBeTruthy();
});
