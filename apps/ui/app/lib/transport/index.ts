export { FramedDaemonTransport } from "./framed";
export { WebSocketDaemonTransport } from "./websocket";
export { TauriDaemonTransport } from "./tauri";
export { TauriFileSource } from "./file-source";
export { TauriLogger } from "./logger";
export { TauriAppData } from "./app-data";
export type { RegistryEntry } from "./app-data";
export { buildDesktopEngineDeps, createDesktopEngine } from "./desktop-engine";
export { bootDesktopHost } from "./desktop-host";
export type { DesktopHost } from "./desktop-host";
export { bindSessionToTerminal } from "./terminal-bind";
export type { TerminalLike } from "./terminal-bind";
export { randomId, hasSecureCrypto } from "./web-crypto";
export {
  bootstrapAgent,
  ensureDaemon,
  assertAgentSupported,
  satisfiesMinVersion,
} from "./host-bootstrap";
export type { AgentInfo, DaemonEnsureResult } from "./host-bootstrap";
export { isDesktopHost, loadTauriCore } from "./core";
export type { TauriCore, TauriChannelLike } from "./tauri";
