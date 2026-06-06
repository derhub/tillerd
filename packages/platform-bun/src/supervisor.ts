import * as fs from "node:fs";
import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { spawnSync } from "node:child_process";
import { DaemonClient } from "./daemon-transport";
import { AtError } from "@athing/sdk";

function getAthingDir(): string {
  return process.env["ATHING_DIR"]
    ? require("node:path").resolve(process.env["ATHING_DIR"])
    : join(homedir(), ".athing");
}

const ATHING_DIR = getAthingDir();
const MANIFEST_PATH = join(ATHING_DIR, "daemon.json");
export const HOOKS_SOCK = join(ATHING_DIR, "hooks.sock");

interface ManifestData {
  pid: number;
  version: string;
}

export function readManifest(manifestPath: string = MANIFEST_PATH): ManifestData | null {
  try {
    return JSON.parse(fs.readFileSync(manifestPath, "utf8")) as ManifestData;
  } catch {
    return null;
  }
}

export function isAlive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

export interface AdoptOrSpawnOptions {
  /** Resolve the daemon binary to spawn. Defaults to the reference-daemon resolver. */
  resolveDaemonBinary?: () => string;
  /** Per-attempt milliseconds to wait for a spawned daemon's control socket. Defaults to 10000. */
  startupTimeoutMs?: number;
  /** Spawn attempts before giving up. Defaults to 3. */
  maxSpawnAttempts?: number;
  /** Base backoff between spawn attempts in ms; doubles each retry. Defaults to 250. */
  spawnBackoffMs?: number;
}

export async function adoptOrSpawn(options: AdoptOrSpawnOptions = {}): Promise<DaemonClient> {
  // Resolve the runtime directory at call time so ATHING_DIR is honored per call
  // (e.g. isolated test directories), not frozen at module load.
  const athingDir = getAthingDir();
  const manifestPath = join(athingDir, "daemon.json");
  const daemonSock = join(athingDir, "daemon.sock");

  const manifest = readManifest(manifestPath);

  if (manifest && isAlive(manifest.pid)) {
    try {
      const client = new DaemonClient(daemonSock);
      await client.connect();
      return client;
    } catch {
      // socket unresponsive — fall through to spawn
    }
  }

  const daemonBin = (options.resolveDaemonBinary ?? resolveDaemonBinary)();
  const timeoutMs = options.startupTimeoutMs ?? 10_000;
  const maxAttempts = options.maxSpawnAttempts ?? 3;
  const backoffMs = options.spawnBackoffMs ?? 250;

  let lastError: AtError | undefined;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    // Clear any stale socket so the freshly spawned daemon binds cleanly.
    try {
      fs.rmSync(daemonSock);
    } catch {
      // no stale socket
    }
    try {
      return await spawnAndConnect(daemonBin, athingDir, daemonSock, timeoutMs);
    } catch (err) {
      lastError = err instanceof AtError ? err : new AtError("SpawnFailed", String(err));
      if (attempt < maxAttempts) {
        await new Promise((r) => setTimeout(r, backoffMs * 2 ** (attempt - 1)));
      }
    }
  }
  throw new AtError(
    "SpawnFailed",
    `Daemon did not start after ${maxAttempts} attempt(s) (${timeoutMs}ms each): ${lastError?.message ?? "unknown error"}`,
  );
}

/** Spawn the daemon once and wait for its control socket to accept a connection. */
async function spawnAndConnect(
  daemonBin: string,
  athingDir: string,
  daemonSock: string,
  timeoutMs: number,
): Promise<DaemonClient> {
  const child = Bun.spawn([daemonBin], {
    detached: true,
    stdio: ["ignore", "ignore", "ignore"],
    // Pin the daemon to the same runtime directory the supervisor resolved, so the
    // two agree on socket/manifest paths regardless of ambient env.
    env: { ...process.env, ATHING_DIR: athingDir },
  });
  child.unref();

  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 100));
    if (fs.existsSync(daemonSock)) {
      try {
        const client = new DaemonClient(daemonSock);
        await client.connect();
        return client;
      } catch {
        // not ready yet
      }
    }
  }
  // Reap the wedged child so a half-started daemon doesn't linger before the next attempt.
  try {
    child.kill();
  } catch {
    // already exited
  }
  throw new AtError("Timeout", `Daemon did not start within ${timeoutMs}ms`);
}

/** Probes the reference resolver reads; injectable so resolution order is testable. */
export interface DaemonResolveProbes {
  env?: Record<string, string | undefined>;
  exists?: (path: string) => boolean;
  cwd?: string;
  home?: string;
  which?: (binary: string) => string | null;
}

export function loginShellWhich(binary: string): string | null {
  const shell = process.env["SHELL"] ?? "/bin/sh";
  const result = spawnSync(shell, ["-lc", `which ${binary}`], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status === 0 && result.stdout.trim()) return result.stdout.trim();
  return null;
}

export function resolveDaemonBinary(probes: DaemonResolveProbes = {}): string {
  const env = probes.env ?? process.env;
  const exists = probes.exists ?? fs.existsSync;
  const cwd = probes.cwd ?? process.cwd();
  const home = probes.home ?? homedir();
  const which = probes.which ?? loginShellWhich;

  const envBin = env["ATHING_DAEMON_BIN"];
  if (envBin) {
    const abs = resolve(envBin);
    if (exists(abs)) return abs;
  }

  const localBin = join(cwd, "bin", "athing-daemon");
  if (exists(localBin)) return localBin;

  const moduleBin = join(import.meta.dir, "../../../bin/athing-daemon");
  if (exists(moduleBin)) return moduleBin;

  const fromShell = which("athing-daemon");
  if (fromShell) return fromShell;

  const userBin = join(home, ".local", "bin", "athing-daemon");
  if (exists(userBin)) return userBin;

  throw new AtError(
    "BinaryNotFound",
    "Cannot resolve athing-daemon binary. Run `bun run build` in packages/daemon-rs or set ATHING_DAEMON_BIN.",
  );
}
