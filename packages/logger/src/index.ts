import pino from "pino";
import type { DestinationStream } from "pino";
import { build as buildPretty } from "pino-pretty";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import type { Logger, LogContext, Resource } from "@tillerd/sdk";

export type { Logger, LogContext, Resource } from "@tillerd/sdk";

const VALID_LEVELS = new Set(["silent", "debug", "info", "warn", "error"]);

function resolveLevel(): string {
  const env = (process.env["LOG_LEVEL"] ?? "info").toLowerCase();
  return VALID_LEVELS.has(env) ? env : "info";
}

function buildDestination(): DestinationStream {
  const tillerdDir = process.env["TILLERD_DIR"];
  const usePretty = process.env["LOG_PRETTY"] === "1";

  const stdoutStream: DestinationStream = usePretty
    ? buildPretty({ colorize: true, sync: true, destination: 1 })
    : (process.stdout as unknown as DestinationStream);

  if (!tillerdDir) return stdoutStream;

  const logsDir = join(tillerdDir, "logs");
  mkdirSync(logsDir, { recursive: true });
  const date = new Date().toISOString().slice(0, 10);
  const fileStream = pino.destination({ dest: join(logsDir, `${date}.log`), sync: true });

  return pino.multistream([{ stream: stdoutStream }, { stream: fileStream }]);
}

function wrap(p: pino.Logger): Logger {
  return {
    debug: (msg, extra) => p.debug(extra ?? {}, msg),
    info: (msg, extra) => p.info(extra ?? {}, msg),
    warn: (msg, extra) => p.warn(extra ?? {}, msg),
    error: (msg, extra) => p.error(extra ?? {}, msg),
    child: (context: LogContext) => wrap(p.child(context)),
  };
}

export function createLogger(resource: Resource): Logger {
  const p = pino(
    {
      level: resolveLevel(),
      formatters: { level: (label) => ({ level: label }) },
      timestamp: () => `,"ts":${Date.now()}`,
      base: resource,
    },
    buildDestination(),
  );

  return wrap(p);
}

export const noopLogger: Logger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
  child: () => noopLogger,
};
