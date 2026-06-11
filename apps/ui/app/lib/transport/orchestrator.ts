import {
  createOrchestratorClient,
  type OrchestratorClient,
  type OrchestratorHostTransport,
  type OrchestratorStatus,
} from "@tillerd/sdk/orchestrator";

export const tauriOrchestratorTransport: OrchestratorHostTransport = {
  async invoke<T>(method: string, args?: Record<string, unknown>): Promise<T> {
    const { invoke } = await import("@tauri-apps/api/core");
    return invoke<T>(method, args);
  },
  async listen(event, handler) {
    const { listen } = await import("@tauri-apps/api/event");
    return listen<OrchestratorStatus>(event, (e) => handler(e.payload));
  },
};

export function createDesktopOrchestratorClient(): OrchestratorClient {
  return createOrchestratorClient(tauriOrchestratorTransport);
}
