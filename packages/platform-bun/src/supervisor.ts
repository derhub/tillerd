import * as fs from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { spawnSync } from "node:child_process";
import { DaemonClient } from "./daemon-transport";

function getAthingDir(): string {
  return process.env["ATHING_DIR"]
    ? require("node:path").resolve(process.env["ATHING_DIR"])
    : join(homedir(), ".athing");
}

const ATHING_DIR = getAthingDir();
const MANIFEST_PATH = join(ATHING_DIR, "daemon.json");
const DAEMON_SOCK = join(ATHING_DIR, "daemon.sock");
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

export async function adoptOrSpawn(): Promise<DaemonClient> {
  const manifest = readManifest();

  if (manifest && isAlive(manifest.pid)) {
    try {
      const client = new DaemonClient(DAEMON_SOCK);
      await client.connect();
      return client;
    } catch {
      // socket unresponsive — fall through to spawn
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

function resolveDaemonBinary(): string {
  const envBin = process.env["ATHING_DAEMON_BIN"];
  if (envBin) {
    const abs = require("node:path").resolve(envBin);
    if (fs.existsSync(abs)) return abs;
  }

  const localBin = join(process.cwd(), "bin", "athing-daemon");
  if (fs.existsSync(localBin)) return localBin;

  const moduleBin = join(import.meta.dir, "../../../../bin/athing-daemon");
  if (fs.existsSync(moduleBin)) return moduleBin;

  const shell = process.env["SHELL"] ?? "/bin/sh";
  const result = spawnSync(shell, ["-lc", "which athing-daemon"], {
    encoding: "utf8",
    timeout: 5000,
  });
  if (result.status === 0 && result.stdout.trim()) return result.stdout.trim();

  const userBin = join(homedir(), ".local", "bin", "athing-daemon");
  if (fs.existsSync(userBin)) return userBin;

  throw new Error(
    "Cannot resolve athing-daemon binary. Run `bun run build` in packages/daemon or set ATHING_DAEMON_BIN.",
  );
}
