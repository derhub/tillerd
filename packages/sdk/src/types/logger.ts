export type AttrValue = string | number | boolean;

/** Structured context bound to a logger; dotted keys map to OpenTelemetry attributes. */
export type LogContext = Record<string, AttrValue>;

/** Per-process identity stamped on every record (the OpenTelemetry Resource concept). */
export interface Resource {
  "service.name": string;
  "service.version": string;
  "service.instance.id"?: string;
  "host.name"?: string;
  "process.pid"?: number;
}

export interface Logger {
  debug(msg: string, extra?: Record<string, unknown>): void;
  info(msg: string, extra?: Record<string, unknown>): void;
  warn(msg: string, extra?: Record<string, unknown>): void;
  error(msg: string, extra?: Record<string, unknown>): void;
  /** Bind context once; the returned logger inherits it on every record. Children compose. */
  child(context: LogContext): Logger;
}
