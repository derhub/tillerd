import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

// The reference host's adopt-or-spawn reaches the spawn path (and the daemon-binary
// resolver) only when no live daemon is recorded. Pointing ATHING_DIR at an empty
// directory guarantees that path, so a throwing resolver proves it was injected.

describe("reference host honors an injected daemon resolver", () => {
  const prevAthingDir = process.env["ATHING_DIR"];
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-rs-test-"));

  afterEach(() => {
    if (prevAthingDir === undefined) delete process.env["ATHING_DIR"];
    else process.env["ATHING_DIR"] = prevAthingDir;
  });

  test("supplied resolver is invoked on the spawn path", async () => {
    process.env["ATHING_DIR"] = tmpDir;
    const { adoptOrSpawn } = await import("@athing/platform-bun");
    await expect(
      adoptOrSpawn({
        resolveDaemonBinary: () => {
          throw new Error("SENTINEL_RESOLVER");
        },
      }),
    ).rejects.toThrow("SENTINEL_RESOLVER");
  });
});
