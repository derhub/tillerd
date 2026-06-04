import { AtError } from "@athing/sdk";
import type { TauriCore } from "./tauri";

export const AGENT_BOOTSTRAP = "agent_bootstrap";
export const DAEMON_ENSURE = "daemon_ensure";

export interface AgentInfo {
  path: string;
  version: string;
  hookCommand: string | null;
  hooksSocketPath: string;
  agentHome: string;
  homeDir: string;
}

export interface DaemonEnsureResult {
  ownership: "owned" | "adopted";
  socket: string;
}

/** Minimal `>=x.y.z` check — the adapter's `cliVersionRange` is expressed this way. */
export function satisfiesMinVersion(version: string, range: string): boolean {
  const min = range.replace(/^>=\s*/, "").trim();
  return comparePep(version, min) >= 0;
}

function comparePep(a: string, b: string): number {
  const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const d = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (d !== 0) return d < 0 ? -1 : 1;
  }
  return 0;
}

/** Throw a typed `VersionUnsupported` error before any session is accepted (§5.5). */
export function assertAgentSupported(version: string, range: string): void {
  if (!satisfiesMinVersion(version, range)) {
    throw new AtError(
      "VersionUnsupported",
      `agent version ${version} does not satisfy ${range}`,
    );
  }
}

/** Resolve the agent via the native core and gate its version against the adapter range. */
export async function bootstrapAgent(core: TauriCore, range: string): Promise<AgentInfo> {
  let info: AgentInfo;
  try {
    info = await core.invoke<AgentInfo>(AGENT_BOOTSTRAP);
  } catch (e) {
    throw new AtError("BinaryNotFound", e instanceof Error ? e.message : String(e));
  }
  assertAgentSupported(info.version, range);
  return info;
}

/** Adopt or spawn the daemon and confirm reachability before sessions start (§5.1-5.3). */
export async function ensureDaemon(core: TauriCore): Promise<DaemonEnsureResult> {
  try {
    return await core.invoke<DaemonEnsureResult>(DAEMON_ENSURE);
  } catch (e) {
    throw new AtError("TransportClosed", e instanceof Error ? e.message : String(e));
  }
}
