import {
  createOrchestratorClient,
  type OrchestratorClient,
  type OrchestratorHostTransport,
  type OrchestratorStatus,
} from "@tillerd/sdk/orchestrator";

// The desktop binding of the SDK's orchestrator host transport: request methods
// go over Tauri `invoke`, the lifecycle stream over the Tauri event channel.
// Tauri modules are imported lazily so this stays inert during SSR/web builds.

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

/** Build the desktop orchestrator client over the Tauri host transport. */
export function createDesktopOrchestratorClient(): OrchestratorClient {
  return createOrchestratorClient(tauriOrchestratorTransport);
}
