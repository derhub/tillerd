import * as fs from "node:fs";
import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { AtError } from "@athing/sdk";

/** The manifest written by a managed tool: pid + version. */
export interface ToolManifest {
  pid: number;
  version: string;
}

/** The spawn-affecting fields for a managed tool process (R6). */
export interface SpawnSpec {
  command: string;
  args: string[];
  cwd?: string;
  env: Record<string, string>;
}

/** The env keys compared when deciding whether to restart (R6). */
export const ENV_ALLOWLIST = [
  "ATHING_DIR",
  "ATHING_GATE_URL",
  "ATHING_SESSION_ID",
  "ATHING_SESSION_TOKEN",
] as const;

/**
 * Returns true when `a` and `b` differ in any spawn-affecting field.
 * Only env keys in `envAllowlist` participate in the comparison (R6).
 */
export function spawnFieldsDiffer(
  a: SpawnSpec,
  b: SpawnSpec,
  envAllowlist: readonly string[] = ENV_ALLOWLIST,
): boolean {
  if (a.command !== b.command) return true;
  if (a.args.length !== b.args.length || a.args.some((v, i) => v !== b.args[i])) return true;
  if ((a.cwd ?? null) !== (b.cwd ?? null)) return true;
  return envAllowlist.some((key) => (a.env[key] ?? undefined) !== (b.env[key] ?? undefined));
}

/** Resolve the ATHING_DIR honoring the env var (R7). */
export function resolveAthingDir(env: Record<string, string | undefined> = process.env): string {
  const raw = env["ATHING_DIR"];
  if (raw) return resolve(raw);
  return join(homedir(), ".athing");
}

function readManifest(manifestPath: string): ToolManifest | null {
  try {
    return JSON.parse(fs.readFileSync(manifestPath, "utf8")) as ToolManifest;
  } catch {
    return null;
  }
}

function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

export interface LaunchOptions {
  /** Expected version for exact-match adoption (R3). */
  version: string;
  /** Full path to the tool manifest file. */
  manifestPath: string;
  /** Spawn attempts before giving up. Defaults to 3. */
  maxAttempts?: number;
  /** Base backoff in ms; doubles each retry: 250ms * 2^(n-1). Defaults to 250. */
  backoffMs?: number;
  /** Per-attempt ms to wait for the manifest to appear after spawn. Defaults to 10000. */
  startupTimeoutMs?: number;
}

/** Outcome of a launch attempt. */
export interface LaunchResult {
  pid: number;
  adopted: boolean;
}

/**
 * Adopt a running tool instance if it matches the exact version (R3), or spawn
 * a new one. Uses bounded exponential backoff (R6) and ATHING_DIR parity (R7).
 */
export async function adoptOrSpawnTool(
  spec: SpawnSpec,
  opts: LaunchOptions,
): Promise<LaunchResult> {
  const { version, manifestPath } = opts;
  const maxAttempts = opts.maxAttempts ?? 3;
  const backoffMs = opts.backoffMs ?? 250;
  const startupTimeoutMs = opts.startupTimeoutMs ?? 10_000;

  // R3: adopt on exact version match
  const existing = readManifest(manifestPath);
  if (existing && isAlive(existing.pid) && existing.version === version) {
    return { pid: existing.pid, adopted: true };
  }

  let lastError: Error | undefined;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const pid = await spawnAndWait(spec, manifestPath, version, startupTimeoutMs);
      return { pid, adopted: false };
    } catch (err) {
      lastError = err instanceof Error ? err : new Error(String(err));
      if (attempt < maxAttempts) {
        await new Promise((r) => setTimeout(r, backoffMs * 2 ** (attempt - 1)));
      }
    }
  }
  throw new AtError(
    "SpawnFailed",
    `Tool did not start after ${maxAttempts} attempt(s): ${lastError?.message ?? "unknown error"}`,
  );
}

async function spawnAndWait(
  spec: SpawnSpec,
  manifestPath: string,
  version: string,
  timeoutMs: number,
): Promise<number> {
  const env: Record<string, string> = { ...(process.env as Record<string, string>), ...spec.env };

  const child = Bun.spawn([spec.command, ...spec.args], {
    detached: true,
    stdio: ["ignore", "ignore", "ignore"],
    cwd: spec.cwd,
    env,
  });
  child.unref();

  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 100));
    const manifest = readManifest(manifestPath);
    if (manifest && isAlive(manifest.pid) && manifest.version === version) {
      return manifest.pid;
    }
  }
  try {
    child.kill();
  } catch {
    // already exited
  }
  throw new AtError("Timeout", `Tool did not write manifest within ${timeoutMs}ms`);
}
