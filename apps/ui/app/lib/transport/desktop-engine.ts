import { createEngine, type EngineDeps } from "@tillerd/engine";
import type { Engine } from "@tillerd/sdk";
import { TauriDaemonTransport } from "./tauri";
import { TauriLogger } from "./logger";
import type { TauriCore } from "./tauri";
import type { AgentInfo } from "./host-bootstrap";

/**
 * Construct the desktop port set from the native core and the host-resolved bootstrap values
 * (§8.2). The engine runs in the web view over these ports; the agent is supplied at
 * `engine.start(adapter, { cwd })`. `cwd` is mandatory — the engine throws a typed
 * `SpawnFailed` when it is absent (§8.3, enforced in the engine proxy).
 */
export function buildDesktopEngineDeps(
  core: TauriCore,
  info: AgentInfo,
  transport: TauriDaemonTransport = new TauriDaemonTransport(core),
): EngineDeps {
  return {
    transport,
    logger: new TauriLogger(core),
    tillerdDir: info.tillerdDir,
    resolvedCommand: info.path,
  };
}

export function createDesktopEngine(core: TauriCore, info: AgentInfo): Engine {
  return createEngine(buildDesktopEngineDeps(core, info));
}
