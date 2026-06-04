import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

// Exercises adoptOrSpawn's spawn-and-wait loop without a real daemon. A no-op resolver
// plus a short startup timeout drive the loop to its timeout. ATHING_DIR is set before a
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

  test("times out when the spawned daemon never exposes a control socket", async () => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-spawn-"));
    process.env["ATHING_DIR"] = dir;

    const { adoptOrSpawn } = await import("../src/supervisor");

    await expect(
      adoptOrSpawn({
        // A binary that exits immediately and never creates the control socket.
        resolveDaemonBinary: () => "/usr/bin/true",
        startupTimeoutMs: 300,
      }),
    ).rejects.toThrow("Daemon did not start within 300ms");
  });
});
