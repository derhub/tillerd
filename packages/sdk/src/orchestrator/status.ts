// Hand-authored minimal wire types for the orchestrator API, centralized in this
// one module. Generating these from `contracts` is deferred to 0.1.4; the surface
// is kept tiny (status + readiness) so the eventual reconciliation is small.
//
// This mirrors the host's `StatusWire` (apps/desktop/src-tauri): an internally
// tagged union on `state`.

/** The orchestrator lifecycle status as observed through the host transport. */
export type OrchestratorStatus =
  | { state: "booting" }
  | { state: "openingStore" }
  | { state: "supervising" }
  | { state: "ready" }
  | { state: "failed"; reason: string };

/** The host command bound to the orchestrator `status()` request method. */
export const ORCHESTRATOR_STATUS_METHOD = "orchestrator_status";

/** The host event name the lifecycle status stream is emitted under. */
export const ORCHESTRATOR_STATUS_EVENT = "orchestrator://status";

/** Whether the orchestrator has reached its ready state. */
export function isReady(status: OrchestratorStatus): boolean {
  return status.state === "ready";
}

/** Whether boot has failed terminally. */
export function isFailed(
  status: OrchestratorStatus,
): status is { state: "failed"; reason: string } {
  return status.state === "failed";
}
