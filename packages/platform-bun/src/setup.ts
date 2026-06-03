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

/** Assemble the setup context the host injects into an adapter's setup procedures. */
export function buildSetupContext(notifyCommand: string, logger: Logger): SetupContext {
  return { notifyCommand, agentHome: agentHome(), logger, fs: setupFs };
}
