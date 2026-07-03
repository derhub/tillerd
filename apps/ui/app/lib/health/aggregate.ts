import type { ServiceHealthWire } from "@tillerd/client-bindings";

export type OrchestratorPhase = "web" | "booting" | "ready" | "error";

export type AggregateState = "ready" | "starting" | "failed";

// versionMismatch counts as failed; draining counts as starting.
export function aggregateHealthState(
  orchestrator: OrchestratorPhase,
  services: ServiceHealthWire[],
): AggregateState {
  if (orchestrator === "error") return "failed";
  if (services.some((s) => s.state === "unavailable" || s.state === "versionMismatch")) {
    return "failed";
  }
  if (orchestrator === "booting") return "starting";
  if (services.some((s) => s.state === "starting" || s.state === "draining")) return "starting";
  return "ready";
}
