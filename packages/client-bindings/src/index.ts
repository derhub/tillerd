// Re-export generated types only. The runtime objects (`commands`, `events`)
// stay internal — consumers use the wrapper (`query`, `command`, ...) below.
export type * from "./tauri_bindings.gen";
export type {
  SessionView as Session,
  ProjectView as Project,
  WorkspaceView as Workspace,
} from "./tauri_bindings.gen";
export {
  query,
  command,
  reorder,
  subscribe,
  entityKey,
  dropById,
  reorderByIds,
  mergeById,
  type CommandKey,
} from "./client.query";
export { setReady, whenReady, ensureResult } from "./readiness";
export { setQueryClient } from "./query-client";
export {
  useEventSub,
  makeSurfaceChannel,
  makeStreamChannel,
  openSurfaceChannel,
  type StreamHandle,
  type ChannelHandle,
  type SurfaceChannelParams,
} from "./subscribe";
