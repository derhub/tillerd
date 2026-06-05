import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { AtError } from "@athing/sdk";

// Exercises adoptOrSpawn's spawn-retry loop without a real daemon. A binary that never
// exposes a control socket drives each attempt to its timeout. ATHING_DIR is set before a
// dynamic import so the supervisor resolves the isolated directory.

describe("adoptOrSpawn spawn path", () => {
  const prevAthingDir = process.env["ATHING_DIR"];
  let dir: string | null = null;

  afterEach(() => {
    if (dir) {
      try {
        fs.rmSync(dir, { recursive: true, force: true });
      } catch {
        // best effort
      }
      dir = null;
    }
    if (prevAthingDir === undefined) delete process.env["ATHING_DIR"];
    else process.env["ATHING_DIR"] = prevAthingDir;
  });

  test("throws a typed SpawnFailed error after exhausting spawn attempts", async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-spawn-"));
    process.env["ATHING_DIR"] = dir;

    const { adoptOrSpawn } = await import("../src/supervisor");

    const error = await adoptOrSpawn({
      // A binary that exits immediately and never creates the control socket.
      resolveDaemonBinary: () => "/usr/bin/true",
      startupTimeoutMs: 150,
      maxSpawnAttempts: 1,
      spawnBackoffMs: 0,
    }).catch((e: unknown) => e);

    expect(error).toBeInstanceOf(AtError);
    expect((error as AtError).kind).toBe("SpawnFailed");
  });

  test("reports the configured attempt ceiling when every spawn fails", async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-spawn-"));
    process.env["ATHING_DIR"] = dir;

    const { adoptOrSpawn } = await import("../src/supervisor");

    const error = await adoptOrSpawn({
      resolveDaemonBinary: () => "/usr/bin/true",
      startupTimeoutMs: 150,
      maxSpawnAttempts: 3,
      spawnBackoffMs: 0,
    }).catch((e: unknown) => e);

    expect((error as AtError).message).toContain("3 attempt(s)");
  });
});
