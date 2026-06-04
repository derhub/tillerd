/**
 * Standardized log attribute and resource keys, shared across runtimes. Dotted names map
 * directly to OpenTelemetry semantic-convention attributes. The Rust daemon mirrors these
 * literals by hand (no cross-language import exists).
 */
export const ATTR = {
  SESSION_ID: "session.id",
  PTY_PID: "pty.pid",
  HOOK_EVENT: "hook.event",
  COMPONENT: "component",
  FRAME_SEQ: "frame.seq",
} as const;

export const RESOURCE_KEY = {
  SERVICE_NAME: "service.name",
  SERVICE_VERSION: "service.version",
  SERVICE_INSTANCE_ID: "service.instance.id",
  HOST_NAME: "host.name",
  PROCESS_PID: "process.pid",
} as const;
