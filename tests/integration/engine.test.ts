import { test, expect, describe, beforeAll, afterAll } from "bun:test";
import { createEngine } from "@tillerd/engine";
import { adoptOrSpawn, resolveTillerdDir, resolveAgentCommand } from "@tillerd/platform-bun";
import { createLogger } from "@tillerd/logger";
import { bashAdapter } from "./fixtures/bash-adapter";
import { startDaemon, type DaemonHandle } from "./fixtures/daemon";

// Provision a daemon in an isolated temp runtime dir and point the host's
// adopt-or-spawn at it (binary + dir), so the engine adopts a live daemon with
// no pre-running service or manually exported env.
let daemon: DaemonHandle;
const priorEnv: Record<string, string | undefined> = {};

beforeAll(async () => {
  daemon = await startDaemon();
  for (const k of ["TILLERD_DIR", "TILLERD_DAEMON_BIN"]) priorEnv[k] = process.env[k];
  process.env["TILLERD_DIR"] = daemon.tillerdDir;
  process.env["TILLERD_DAEMON_BIN"] = daemon.bin;
});

afterAll(async () => {
  for (const [k, v] of Object.entries(priorEnv)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
  await daemon?.stop();
});

async function makeEngine() {
  const transport = await adoptOrSpawn();
  return createEngine({
    transport,
    logger: createLogger({ "service.name": "tillerd-integration-test", "service.version": "0" }),
    tillerdDir: resolveTillerdDir(),
    resolvedCommand: resolveAgentCommand(bashAdapter.binaryResolution),
  });
}

describe("engine integration", () => {
  test("start → ready → onData → kill", async () => {
    const engine = await makeEngine();
    const dataChunks: Uint8Array[] = [];

    const session = await engine.start(bashAdapter, {
      cwd: process.cwd(),
      startupTimeoutMs: 5_000,
    });
    session.onData((b) => dataChunks.push(b));

    await new Promise<void>((resolve) => {
      const unsub = session.onStatus((s) => {
        if (s === "IDLE") {
          unsub();
          resolve();
        }
      });
      setTimeout(resolve, 3_000);
    });

    expect(dataChunks.length).toBeGreaterThan(0);

    await session.kill();
    await engine.shutdown();
  }, 10_000);

  test("two concurrent sessions are isolated", async () => {
    const engine = await makeEngine();
    const [s1, s2] = await Promise.all([
      engine.start(bashAdapter, { cwd: process.cwd(), startupTimeoutMs: 5_000 }),
      engine.start(bashAdapter, { cwd: process.cwd(), startupTimeoutMs: 5_000 }),
    ]);

    expect(s1.sessionId).not.toBe(s2.sessionId);

    await Promise.all([s1.kill(), s2.kill()]);
    await engine.shutdown();
  }, 15_000);

  test("BinaryNotFound when binary missing", async () => {
    const transport = await adoptOrSpawn();
    const engine = createEngine({
      transport,
      logger: createLogger({ "service.name": "tillerd-integration-test", "service.version": "0" }),
      tillerdDir: resolveTillerdDir(),
      resolvedCommand: "__tillerd_no_such_binary__",
    });

    const errors: string[] = [];
    const session = await engine.start(bashAdapter, {
      cwd: process.cwd(),
      startupTimeoutMs: 5_000,
    });
    session.onError((e) => errors.push(e.kind));

    await new Promise((r) => setTimeout(r, 500));
    expect(errors.some((k) => k === "BinaryNotFound" || k === "SpawnFailed")).toBe(true);

    await engine.shutdown();
  }, 10_000);
});
