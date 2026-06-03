import pino from "pino";
import type { DestinationStream } from "pino";
import { build as buildPretty } from "pino-pretty";
import { mkdirSync } from "node:fs";
import { join } from "node:path";

export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogEntry {
  ts: number;
  level: LogLevel;
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

const VALID_LEVELS = new Set(["silent", "debug", "info", "warn", "error"]);

function resolveLevel(): string {
  const env = (process.env["LOG_LEVEL"] ?? "info").toLowerCase();
  return VALID_LEVELS.has(env) ? env : "info";
}

function buildDestination(): DestinationStream {
  const athingDir = process.env["ATHING_DIR"];
  const usePretty = process.env["LOG_PRETTY"] === "1";

  const stdoutStream: DestinationStream = usePretty
    ? buildPretty({ colorize: true, sync: true, destination: 1 })
    : (process.stdout as unknown as DestinationStream);

  if (!athingDir) return stdoutStream;

  const logsDir = join(athingDir, "logs");
  mkdirSync(logsDir, { recursive: true });
  const date = new Date().toISOString().slice(0, 10);
  const fileStream = pino.destination({ dest: join(logsDir, `${date}.log`), sync: true });

  return pino.multistream([{ stream: stdoutStream }, { stream: fileStream }]);
}

export function createLogger(sessionId?: string): Logger {
  const p = pino(
    {
      level: resolveLevel(),
      formatters: { level: (label) => ({ level: label }) },
      timestamp: () => `,"ts":${Date.now()}`,
      base: undefined,
    },
    buildDestination(),
  );

  const child = sessionId !== undefined ? p.child({ sessionId }) : p;

  return {
    debug: (msg, extra) => child.debug(extra ?? {}, msg),
    info: (msg, extra) => child.info(extra ?? {}, msg),
    warn: (msg, extra) => child.warn(extra ?? {}, msg),
    error: (msg, extra) => child.error(extra ?? {}, msg),
  };
}

export const noopLogger: Logger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
};
