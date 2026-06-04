import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

// Integration: drives the real native daemon binary end to end through the native
// host's supervision — spawn, wire handshake, control-plane list. Skipped unless the
// binary has been built (cargo build --release in packages/daemon-rs).

const NATIVE_BIN = path.join(import.meta.dir, "../../daemon-rs/target/release/athing-daemon");
const built = fs.existsSync(NATIVE_BIN);

/**
 * The daemon writes its manifest shortly after the control socket begins serving,
 * so a fast client can connect before the pid is recorded. Poll briefly to read it
 * for teardown.
 */
async function readDaemonPid(dir: string): Promise<number | null> {
  for (let attempt = 0; attempt < 40; attempt++) {
    try {
      const manifest = JSON.parse(fs.readFileSync(path.join(dir, "daemon.json"), "utf8")) as {
        pid: number;
      };
      return manifest.pid;
    } catch {
      await Bun.sleep(50);
    }
  }
  return null;
}

describe.skipIf(!built)("native host drives the real native daemon", () => {
  const prevAthingDir = process.env["ATHING_DIR"];
  const prevDaemonBin = process.env["ATHING_DAEMON_BIN"];
  let athingDir: string | null = null;

  afterEach(async () => {
    if (athingDir) {
      const pid = await readDaemonPid(athingDir);
      if (pid !== null) {
        try {
          process.kill(pid, "SIGTERM");
        } catch {
          // already gone
        }
      }
      try {
        fs.rmSync(athingDir, { recursive: true, force: true });
      } catch {
        // best effort
      }
      athingDir = null;
    }
    if (prevAthingDir === undefined) delete process.env["ATHING_DIR"];
    else process.env["ATHING_DIR"] = prevAthingDir;
    if (prevDaemonBin === undefined) delete process.env["ATHING_DAEMON_BIN"];
    else process.env["ATHING_DAEMON_BIN"] = prevDaemonBin;
  });

  test(
    "spawns the daemon, completes the handshake, and lists sessions",
    async () => {
      athingDir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-rs-live-"));
      process.env["ATHING_DIR"] = athingDir;
      process.env["ATHING_DAEMON_BIN"] = NATIVE_BIN;

      const { adoptOrSpawn } = await import("../src/index");
      const client = await adoptOrSpawn();

      try {
        // adoptOrSpawn resolving means the version/capability handshake completed;
        // a list round-trip confirms the control plane answers.
        const ids = await client.list();
        expect(ids).toEqual([]);
      } finally {
        client.disconnect();
      }
    },
    15_000,
  );

  test(
    "adopts an already-running daemon instead of spawning a second",
    async () => {
      athingDir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-rs-live-"));
      process.env["ATHING_DIR"] = athingDir;
      process.env["ATHING_DAEMON_BIN"] = NATIVE_BIN;

      const { adoptOrSpawn } = await import("../src/index");

      const first = await adoptOrSpawn();
      first.disconnect();

      // Ensure the manifest is recorded so the next call takes the adopt path.
      const pid = await readDaemonPid(athingDir);
      expect(pid).not.toBeNull();

      const second = await adoptOrSpawn();
      try {
        expect(await second.list()).toEqual([]);
        // Same daemon: adopted, not respawned.
        expect(await readDaemonPid(athingDir)).toBe(pid);
      } finally {
        second.disconnect();
      }
    },
    15_000,
  );
});
