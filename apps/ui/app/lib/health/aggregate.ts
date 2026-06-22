import type { ServiceHealth } from "@tillerd/sdk/orchestrator";

/** The orchestrator's own boot phase, as the host hook reports it. */
export type OrchestratorPhase = "web" | "booting" | "ready" | "error";

/** The single aggregate indicator's state -- the worst across orchestrator + services. */
export type AggregateState = "ready" | "starting" | "failed";

/**
 * Reduce the orchestrator phase and every service's state to one indicator state:
 * `failed` if anything is broken, else `starting` while anything is coming up,
 * else `ready`. Version-mismatch counts as failed (a service stuck on the wrong
 * version); draining counts as starting (a transient drain-restart).
 */
export function aggregateHealthState(
  orchestrator: OrchestratorPhase,
  services: ServiceHealth[],
): AggregateState {
  if (orchestrator === "error") return "failed";
  if (services.some((s) => s.state === "unavailable" || s.state === "versionMismatch")) {
    return "failed";
  }
  if (orchestrator === "booting") return "starting";
  if (services.some((s) => s.state === "starting" || s.state === "draining")) return "starting";
  return "ready";
}
