export interface LogEntry {
  ts: number;
  level: "debug" | "info" | "warn" | "error";
  sessionId?: string;
  msg: string;
  [key: string]: unknown;
}

export interface Logger {
  debug(msg: string, extra?: Record<string, unknown>): void;
  info(msg: string, extra?: Record<string, unknown>): void;
  warn(msg: string, extra?: Record<string, unknown>): void;
  error(msg: string, extra?: Record<string, unknown>): void;
}

export function createLogger(sessionId?: string): Logger {
  function emit(level: LogEntry["level"], msg: string, extra?: Record<string, unknown>) {
    const entry: LogEntry = { ts: Date.now(), level, sessionId, msg, ...extra };
    const out = JSON.stringify(entry);
    if (level === "error" || level === "warn") {
      process.stderr.write(out + "\n");
    } else {
      process.stdout.write(out + "\n");
    }
  }

  return {
    debug: (msg, extra) => emit("debug", msg, extra),
    info: (msg, extra) => emit("info", msg, extra),
    warn: (msg, extra) => emit("warn", msg, extra),
    error: (msg, extra) => emit("error", msg, extra),
  };
}

export const noopLogger: Logger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
};
