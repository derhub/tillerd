import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { AtError } from "@athing/sdk";

// The native host's adopt-or-spawn reaches the daemon resolver only on the spawn
// path; an empty ATHING_DIR guarantees that path (no manifest to adopt). Forwarded
// resolution probes then control the native resolver at its filesystem boundary, so
// the wiring is observable without a built daemon or a live socket.

describe("native host wires the native daemon resolver", () => {
  const prevAthingDir = process.env["ATHING_DIR"];

  afterEach(() => {
    if (prevAthingDir === undefined) delete process.env["ATHING_DIR"];
    else process.env["ATHING_DIR"] = prevAthingDir;
  });

  test("defaults to the native resolver and fails with the cargo build hint when unresolvable", async () => {
    process.env["ATHING_DIR"] = fs.mkdtempSync(path.join(os.tmpdir(), "athing-rs-wire-"));
    const { adoptOrSpawn } = await import("../src/index");

    let thrown: unknown;
    try {
      await adoptOrSpawn({ probes: { env: {}, exists: () => false } });
    } catch (err) {
      thrown = err;
    }

    expect(thrown).toBeInstanceOf(AtError);
    expect((thrown as AtError).kind).toBe("BinaryNotFound");
    expect((thrown as AtError).message).toContain("cargo build");
    expect((thrown as AtError).message).toContain("ATHING_DAEMON_BIN");
  });

  test("forwards resolution probes to the native resolver", async () => {
    process.env["ATHING_DIR"] = fs.mkdtempSync(path.join(os.tmpdir(), "athing-rs-wire-"));
    const { adoptOrSpawn } = await import("../src/index");

    await expect(
      adoptOrSpawn({
        probes: {
          env: {},
          exists: () => {
            throw new Error("PROBE_CONSULTED");
          },
        },
      }),
    ).rejects.toThrow("PROBE_CONSULTED");
  });
});
