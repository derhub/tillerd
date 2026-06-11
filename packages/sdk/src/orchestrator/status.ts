export type OrchestratorStatus =
  | { state: "booting" }
  | { state: "openingStore" }
  | { state: "supervising" }
  | { state: "ready" }
  | { state: "failed"; reason: string };

export const ORCHESTRATOR_STATUS_METHOD = "orchestrator_status";

export const ORCHESTRATOR_STATUS_EVENT = "orchestrator://status";

export function isReady(status: OrchestratorStatus): boolean {
  return status.state === "ready";
}

export function isFailed(
  status: OrchestratorStatus,
): status is { state: "failed"; reason: string } {
  return status.state === "failed";
}
