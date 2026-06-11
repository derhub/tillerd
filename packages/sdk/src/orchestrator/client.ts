import type { OrchestratorStatus } from "./status";
import { ORCHESTRATOR_STATUS_EVENT, ORCHESTRATOR_STATUS_METHOD } from "./status";

// The transport-agnostic SDK client of the orchestrator API. It carries no
// backend logic: it only routes typed request/response methods and event
// subscriptions through the host transport the renderer binds (the desktop binds
// it to Tauri commands + the event channel). It synthesizes no events of its own.

/**
 * The host transport the client calls. The host (desktop renderer) binds this to
 * its own transport — invoking commands and subscribing to the event channel.
 */
export interface OrchestratorHostTransport {
  /** Invoke a request/response method on the orchestrator API. */
  invoke<T>(method: string, args?: Record<string, unknown>): Promise<T>;
  /** Subscribe to a host event channel; resolves to an unsubscribe function. */
  listen(
    event: string,
    handler: (payload: OrchestratorStatus) => void,
  ): Promise<() => void>;
}

/** A typed client of the orchestrator API. */
export interface OrchestratorClient {
  /** Request the orchestrator's current lifecycle status. */
  status(): Promise<OrchestratorStatus>;
  /**
   * Subscribe to lifecycle status events. Resolves to an unsubscribe function.
   * Events are delivered exactly as the orchestrator emits them.
   */
  subscribe(
    handler: (status: OrchestratorStatus) => void,
  ): Promise<() => void>;
}

/** Build a typed orchestrator client over a host transport. */
export function createOrchestratorClient(
  transport: OrchestratorHostTransport,
): OrchestratorClient {
  return {
    status: () => transport.invoke<OrchestratorStatus>(ORCHESTRATOR_STATUS_METHOD),
    subscribe: (handler) => transport.listen(ORCHESTRATOR_STATUS_EVENT, handler),
  };
}
