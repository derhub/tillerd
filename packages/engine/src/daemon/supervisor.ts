import * as fs from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { spawnSync } from "node:child_process";
import { DaemonClient } from "./client";
import { DAEMON_VERSION } from "@athing/daemon";

function getAthingDir(): string {
  return process.env["ATHING_DIR"]
    ? require("node:path").resolve(process.env["ATHING_DIR"])
    : join(homedir(), ".athing");
}

const ATHING_DIR = getAthingDir();
const MANIFEST_PATH = join(ATHING_DIR, "daemon.json");
const DAEMON_SOCK = join(ATHING_DIR, "daemon.sock");

interface ManifestData {
  pid: number;
  version: string;
}

function readManifest(): ManifestData | null {
  try {
    return JSON.parse(fs.readFileSync(MANIFEST_PATH, "utf8")) as ManifestData;
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

export async function adoptOrSpawn(): Promise<DaemonClient> {
  const manifest = readManifest();

  if (manifest && isAlive(manifest.pid)) {
    if (manifest.version !== DAEMON_VERSION) {
      // Attempt a live upgrade via the wire protocol before falling back to kill.
      try {
        const client = new DaemonClient(DAEMON_SOCK);
        await client.connect();
        await triggerUpgrade(client);
        client.disconnect();
        // Wait for the predecessor to exit and successor to bind the socket.
        await waitForNewDaemon(DAEMON_SOCK, manifest.pid, 12_000);
      } catch {
        // Upgrade attempt failed — fall back to SIGTERM.
        try {
          process.kill(manifest.pid, "SIGTERM");
        } catch {
          /* already dead */
        }
        await new Promise((r) => setTimeout(r, 500));
      }
    } else {
      try {
        const client = new DaemonClient(DAEMON_SOCK);
        await client.connect();
        return client;
      } catch {
        // socket unresponsive — fall through to spawn
      }
    }
  }

  try {
    fs.rmSync(DAEMON_SOCK);
  } catch {
    // no stale socket
  }

  const daemonBin = resolveDaemonBinary();
  const child = Bun.spawn([daemonBin], {
    detached: true,
    stdio: ["ignore", "ignore", "ignore"],
  });
  child.unref();

  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 100));
    if (fs.existsSync(DAEMON_SOCK)) {
      try {
        const client = new DaemonClient(DAEMON_SOCK);
        await client.connect();
        return client;
      } catch {
        // not ready yet
      }
    }
  }
  throw new Error("Daemon did not start within 10 seconds");
}

/** Send an upgrade frame to the running daemon, initiating the handoff sequence. */
async function triggerUpgrade(client: DaemonClient): Promise<void> {
  client.send({ type: "upgrade" });
}

/**
 * Wait until the daemon socket is held by a new process (different pid from oldPid),
 * or until the timeout elapses.
 */
async function waitForNewDaemon(
  sockPath: string,
  oldPid: number,
  timeoutMs: number,
): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 200));
    if (!fs.existsSync(sockPath)) continue;
    const manifest = readManifest();
    if (manifest && manifest.pid !== oldPid && isAlive(manifest.pid)) return;
  }
  throw new Error("Daemon upgrade did not complete within timeout");
}

function resolveDaemonBinary(): string {
  // Explicit override
  const envBin = process.env["ATHING_DAEMON_BIN"];
  if (envBin) {
    const abs = require("node:path").resolve(envBin);
    if (fs.existsSync(abs)) return abs;
  }

  // Project-local bin/ relative to cwd (dev mode)
  const localBin = join(process.cwd(), "bin", "athing-daemon");
  if (fs.existsSync(localBin)) return localBin;

  // Project-local bin/ relative to this module (works when cwd != repo root)
  const moduleBin = join(import.meta.dir, "../../../../bin/athing-daemon");
  if (fs.existsSync(moduleBin)) return moduleBin;

  // Login-shell PATH
  const shell = process.env["SHELL"] ?? "/bin/sh";
  const result = spawnSync(shell, ["-lc", "which athing-daemon"], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status === 0 && result.stdout.trim()) return result.stdout.trim();

  // User-local install
  const userBin = join(homedir(), ".local", "bin", "athing-daemon");
  if (fs.existsSync(userBin)) return userBin;

  throw new Error(
    "Cannot resolve athing-daemon binary. Run `bun run build` in packages/daemon or set ATHING_DAEMON_BIN.",
  );
}
