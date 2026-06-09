import * as fs from "node:fs";
import * as path from "node:path";
import type { Logger, SetupContext, SetupFs } from "@athing/sdk";
import { setup } from "@athing/adapter-claude-code";
import type { CliDeps, ManifestData } from "../src/cli";

const FIXTURES = path.join(import.meta.dir, "fixtures");
export const AGENT_HOME = "/agent/.claude";
export const SETTINGS = `${AGENT_HOME}/settings.json`;
export const NOTIFY = "/fake/bin/athing-notify";

/** In-memory SetupFs seeded from a hand-authored fixture — never the real settings file. */
export function fakeFs(fixture?: string): { fs: SetupFs; files: Map<string, string> } {
  const files = new Map<string, string>();
  if (fixture) files.set(SETTINGS, fs.readFileSync(path.join(FIXTURES, fixture), "utf8"));
  const cap: SetupFs = {
    async readText(p) {
      return files.has(p) ? files.get(p)! : null;
    },
    async writeAtomic(p, text) {
      files.set(p, text);
    },
    async backup() {},
    async exists(p) {
      return files.has(p);
    },
  };
  return { fs: cap, files };
}

export interface HarnessOverrides {
  fixture?: string;
  manifest?: ManifestData | null;
  isAlive?: boolean;
  isTTY?: boolean;
  confirmResult?: boolean;
  resolveNotify?: () => string;
}

export interface Harness {
  deps: CliDeps;
  files: Map<string, string>;
  out: string[];
  err: string[];
  confirmCalls: number;
}

export function harness(o: HarnessOverrides = {}): Harness {
  const { fs: cap, files } = fakeFs(o.fixture);
  const out: string[] = [];
  const err: string[] = [];
  let confirmCalls = 0;

  const deps: CliDeps = {
    setup,
    buildContext: (notifyCommand: string, logger: Logger): SetupContext => ({
      notifyCommand,
      agentHome: AGENT_HOME,
      logger,
      fs: cap,
    }),
    resolveNotify: o.resolveNotify ?? (() => NOTIFY),
    readManifest: () => o.manifest ?? null,
    isAlive: () => o.isAlive ?? false,
    isTTY: o.isTTY ?? false,
    async confirm() {
      confirmCalls += 1;
      return o.confirmResult ?? false;
    },
    out: (line) => out.push(line),
    err: (line) => err.push(line),
  };

  return {
    deps,
    files,
    out,
    err,
    get confirmCalls() {
      return confirmCalls;
    },
  };
}

export function settings(files: Map<string, string>): {
  hooks?: Record<string, Array<{ matcher: string; hooks: Array<{ command: string }> }>>;
  [k: string]: unknown;
} {
  return JSON.parse(files.get(SETTINGS)!);
}

export function commandsFor(s: ReturnType<typeof settings>, event: string): string[] {
  return (s.hooks?.[event] ?? []).flatMap((e) => e.hooks.map((h) => h.command));
}
