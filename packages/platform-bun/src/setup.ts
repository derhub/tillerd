import * as fs from "node:fs";
import * as path from "node:path";
import { homedir } from "node:os";
import type { Logger, SetupContext, SetupFs } from "@athing/sdk";

/** The agent-home location the host resolves once at startup. */
export function agentHome(): string {
  return path.join(homedir(), ".claude");
}

/**
 * The generic filesystem mechanics an adapter's setup procedures call. Backup and
 * atomic write live here once and are reused by every adapter.
 */
export const setupFs: SetupFs = {
  async readText(p) {
    try {
      return fs.readFileSync(p, "utf8");
    } catch {
      return null;
    }
  },

  async writeAtomic(p, text) {
    fs.mkdirSync(path.dirname(p), { recursive: true });
    const tmp = `${p}.athing-tmp`;
    fs.writeFileSync(tmp, text, "utf8");
    fs.renameSync(tmp, p);
  },

  async backup(p) {
    if (!fs.existsSync(p)) return;
    const ts = new Date()
      .toISOString()
      .replace(/:/g, "-")
      .replace(/\.\d+Z$/, "Z");
    fs.copyFileSync(p, `${p}.athing-backup-${ts}`);
  },

  async exists(p) {
    return fs.existsSync(p);
  },
};

/**
 * Resolve the gate URL: ATHING_GATE_URL env var, else $ATHING_DIR/gate.url file,
 * else undefined.
 */
export function resolveGateUrl(
  env: Record<string, string | undefined> = process.env,
): string | undefined {
  const fromEnv = env["ATHING_GATE_URL"];
  if (fromEnv) return fromEnv;

  const athingDir = env["ATHING_DIR"]
    ? path.resolve(env["ATHING_DIR"])
    : path.join(homedir(), ".athing");
  const urlFile = path.join(athingDir, "gate.url");
  try {
    const content = fs.readFileSync(urlFile, "utf8").trim();
    return content || undefined;
  } catch {
    return undefined;
  }
}

export interface BuildSetupContextOptions {
  gateUrl?: string;
  sessionId?: string;
  sessionToken?: string;
}

/** Assemble the setup context the host injects into an adapter's setup procedures. */
export function buildSetupContext(
  notifyCommand: string,
  logger: Logger,
  opts: BuildSetupContextOptions = {},
): SetupContext {
  return {
    notifyCommand,
    agentHome: agentHome(),
    logger,
    fs: setupFs,
    ...(opts.gateUrl !== undefined && { gateUrl: opts.gateUrl }),
    ...(opts.sessionId !== undefined && { sessionId: opts.sessionId }),
    ...(opts.sessionToken !== undefined && { sessionToken: opts.sessionToken }),
  };
}
