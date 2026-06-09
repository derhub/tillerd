export { DaemonClient } from "./daemon-transport";
export { adoptOrSpawn, resolveDaemonBinary, readManifest, isAlive } from "./supervisor";
export type { AdoptOrSpawnOptions, DaemonResolveProbes } from "./supervisor";
export { checkCliVersion, resolveAgentCommand } from "./resolve";
export { prepareNotifyScript, notifyCommand, notifyScriptPath } from "./ingress";
export { BunFileSource } from "./file-source";
export { agentHome, setupFs, buildSetupContext } from "./setup";
export {
  adoptOrSpawnTool,
  spawnFieldsDiffer,
  resolveTillerdDir,
  ENV_ALLOWLIST,
} from "./process-launch";
export type { SpawnSpec, ToolManifest, LaunchOptions, LaunchResult } from "./process-launch";
