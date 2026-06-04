import { adoptOrSpawn as adoptOrSpawnBase, type DaemonClient } from "@athing/platform-bun";
import { resolveNativeDaemonBinary, type DaemonResolveProbes } from "./resolve-daemon";

export interface NativeAdoptOrSpawnOptions {
  /** Resolution probes forwarded to the native daemon resolver (for tests/diagnostics). */
  probes?: DaemonResolveProbes;
}

/**
 * Adopt a live native daemon or spawn one, defaulting the backend to the native
 * build. Reuses the reference host's supervision body, supplying the native
 * binary resolver.
 */
export function adoptOrSpawn(options: NativeAdoptOrSpawnOptions = {}): Promise<DaemonClient> {
  return adoptOrSpawnBase({
    resolveDaemonBinary: () => resolveNativeDaemonBinary(options.probes),
  });
}

export { resolveNativeDaemonBinary };
export type { DaemonResolveProbes };

// Backend-independent platform-port surface, reused unchanged from the reference host.
export {
  DaemonClient,
  readManifest,
  isAlive,
  HOOKS_SOCK,
  checkCliVersion,
  resolveAgentCommand,
  prepareNotifyScript,
  notifyCommand,
  notifyScriptPath,
  BunFileSource,
  agentHome,
  setupFs,
  buildSetupContext,
} from "@athing/platform-bun";
