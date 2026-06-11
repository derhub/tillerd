export { createOrchestratorClient } from "./client";
export type { OrchestratorClient, OrchestratorHostTransport } from "./client";
export { ORCHESTRATOR_STATUS_EVENT, ORCHESTRATOR_STATUS_METHOD, isFailed, isReady } from "./status";
export type { OrchestratorStatus } from "./status";
export {
  SURFACE_CREATE,
  SURFACE_INPUT,
  SURFACE_RESIZE,
  SURFACE_DETACH,
  SURFACE_STATUS_EVENT,
  SURFACE_EXIT_EVENT,
  createTerminalSurfaceClient,
} from "./terminal-surface";
export type {
  SurfaceStatusEvent,
  SurfaceExitEvent,
  TerminalSurfaceTransport,
  CreateTerminalOptions,
  TerminalSurfaceClient,
} from "./terminal-surface";
