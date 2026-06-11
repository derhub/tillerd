export type * from "./types/index";
export {
  AtError,
  defineSetup,
  HOOK_SUBSCRIPTION_WIRE_VERSION,
  RawFrame,
  SubscriptionFrameDecoder,
  encodeSubscriptionFrame,
  encodeSubscribePreamble,
  decodeSubscriptionFrame,
  negotiateReady,
} from "./types/index";
export * from "./protocol/index";
export { resolveSignal, signalCategoryToQualifier } from "./signals";
export type { SignalInfo, ResolvedSignal } from "./signals";
export { exitToStatus, isRecoverable, qualifierToCoarse } from "./exit-qualifier";
export { encodeKey, encodeKeySequence } from "./keys";
export type { KeyEncodeOptions } from "./keys";
export type { DaemonTransport, FileSource, FrameHandler, HookSource } from "./ports";
export { ATTR, RESOURCE_KEY } from "./attributes";
export { hookEventToContent } from "./hook-content";
export {
  SURFACE_CREATE,
  SURFACE_INPUT,
  SURFACE_RESIZE,
  SURFACE_DETACH,
  SURFACE_STATUS_EVENT,
  SURFACE_EXIT_EVENT,
  createTerminalSurfaceClient,
} from "./orchestrator/terminal-surface";
export type {
  SurfaceStatusEvent,
  SurfaceExitEvent,
  TerminalSurfaceTransport,
  CreateTerminalOptions,
  TerminalSurfaceClient,
} from "./orchestrator/terminal-surface";
