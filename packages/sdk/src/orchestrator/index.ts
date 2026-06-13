export { createOrchestratorClient } from "./client";
export type {
  OrchestratorClient,
  OrchestratorHostTransport,
  Project,
  Session,
  CreateProjectArgs,
  RenameProjectArgs,
  ArchiveProjectArgs,
  CreateSessionArgs,
  RenameSessionArgs,
  ListSessionsArgs,
  ArchiveSessionArgs,
  SetSessionLayoutArgs,
  GetSessionLayoutArgs,
} from "./client";
export { ORCHESTRATOR_STATUS_EVENT, ORCHESTRATOR_STATUS_METHOD, isFailed, isReady } from "./status";
export type { OrchestratorStatus } from "./status";
export {
  SURFACE_CREATE,
  SURFACE_SPAWN,
  SURFACE_CLOSE,
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
export {
  PROJECT_CREATE,
  PROJECT_RENAME,
  PROJECT_LIST,
  PROJECT_ARCHIVE,
  SESSION_CREATE,
  SESSION_RENAME,
  SESSION_LIST,
  SESSION_ARCHIVE,
  SESSION_LAYOUT_SET,
  SESSION_LAYOUT_GET,
  createWorkspaceClient,
} from "./workspace";
export type { SourceKind, TitleSource, WorkspaceClient, WorkspaceTransport } from "./workspace";
