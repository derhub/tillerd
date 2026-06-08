import type { Logger, LogContext } from "@athing/sdk";
import type { TauriCore } from "./tauri";

export const LOG_FORWARD = "log_forward";

type Level = "debug" | "info" | "warn" | "error";

/**
 * Native (web-view) `Logger`: writes to the console and optionally forwards to the Rust core for
 * file/diagnostic capture. Forwarding is best-effort and never throws into the caller.
 */
export class TauriLogger implements Logger {
  constructor(
    private readonly core?: TauriCore,
    private readonly bindings: LogContext = {},
  ) {}

  debug(msg: string, extra?: Record<string, unknown>): void {
    this.emit("debug", msg, extra);
  }
  info(msg: string, extra?: Record<string, unknown>): void {
    this.emit("info", msg, extra);
  }
  warn(msg: string, extra?: Record<string, unknown>): void {
    this.emit("warn", msg, extra);
  }
  error(msg: string, extra?: Record<string, unknown>): void {
    this.emit("error", msg, extra);
  }

  child(context: LogContext): Logger {
    return new TauriLogger(this.core, { ...this.bindings, ...context });
  }

  private emit(level: Level, msg: string, extra?: Record<string, unknown>): void {
    const merged = { ...this.bindings, ...extra };
    const hasFields = Object.keys(merged).length > 0;
    const sink = level === "debug" ? console.debug : console[level];
    if (hasFields) sink(msg, merged);
    else sink(msg);
    if (this.core) {
      void this.core
        .invoke(LOG_FORWARD, { level, msg, extra: hasFields ? merged : null })
        .catch(() => {});
    }
  }
}
