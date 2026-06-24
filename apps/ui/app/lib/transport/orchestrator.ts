import { commands, events, type StatusWire } from "@tillerd/client-bindings";

export type { StatusWire };

export interface SimpleOrchestratorClient {
  subscribe(handler: (status: StatusWire) => void): Promise<() => void>;
  status(): Promise<StatusWire>;
}

export function createDesktopOrchestratorClient(): SimpleOrchestratorClient {
  return {
    subscribe: (handler) => events.orchestratorStatus.listen((e) => handler(e.payload)),
    status: () => commands.orchestratorStatus(),
  };
}
