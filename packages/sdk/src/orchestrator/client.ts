import type { OrchestratorStatus } from "./status";
import { ORCHESTRATOR_STATUS_EVENT, ORCHESTRATOR_STATUS_METHOD } from "./status";

export interface OrchestratorHostTransport {
  invoke<T>(method: string, args?: Record<string, unknown>): Promise<T>;
  listen(event: string, handler: (payload: OrchestratorStatus) => void): Promise<() => void>;
}

export interface OrchestratorClient {
  status(): Promise<OrchestratorStatus>;
  subscribe(handler: (status: OrchestratorStatus) => void): Promise<() => void>;
}

export function createOrchestratorClient(transport: OrchestratorHostTransport): OrchestratorClient {
  return {
    status: () => transport.invoke<OrchestratorStatus>(ORCHESTRATOR_STATUS_METHOD),
    subscribe: (handler) => transport.listen(ORCHESTRATOR_STATUS_EVENT, handler),
  };
}
