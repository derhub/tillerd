import { createEngine } from "@athing/engine";
import type { AgentDefinition, Engine } from "@athing/sdk";
import { claudeCode, SUPPORTED_CLI_VERSION_RANGE } from "@athing/adapter-claude-code";
import type { TauriCore } from "./tauri";
import { TauriDaemonTransport } from "./tauri";
import type { AgentInfo } from "./host-bootstrap";
import { bootstrapAgent, ensureDaemon } from "./host-bootstrap";
import { buildDesktopEngineDeps } from "./desktop-engine";
import { TauriAppData } from "./app-data";

export interface DesktopHost {
  engine: Engine;
  agent: AgentDefinition;
  info: AgentInfo;
}

/**
 * Desktop app-boot sequence (§8): resolve + version-gate the agent, adopt/spawn the daemon,
 * connect the native byte-bridge transport, construct the renderer engine, and reconcile the
 * session registry against the daemon's live sessions (§6.3). Throws typed errors
 * (`VersionUnsupported`, `BinaryNotFound`, `TransportClosed`) before any session is accepted.
 * Hook installation is NOT a desktop concern — the CLI client owns it.
 */
export async function bootDesktopHost(core: TauriCore): Promise<DesktopHost> {
  const info = await bootstrapAgent(core, SUPPORTED_CLI_VERSION_RANGE);
  await ensureDaemon(core);

  const transport = new TauriDaemonTransport(core);
  await transport.connect();
  const engine = createEngine(buildDesktopEngineDeps(core, info, transport));

  try {
    await new TauriAppData(core).reconcile(await engine.listSessions());
  } catch {
    // best-effort: a stale registry must not block boot
  }

  return { engine, agent: claudeCode, info };
}
