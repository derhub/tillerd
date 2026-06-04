import { test, expect, describe } from "bun:test";
import { createEngine } from "@athing/engine";
import { adoptOrSpawn, agentHome, BunFileSource, HOOKS_SOCK, resolveAgentCommand } from "@athing/platform-bun";
import { createLogger } from "@athing/logger";
import { bashAdapter } from "./fixtures/bash-adapter";

async function makeEngine() {
  const transport = await adoptOrSpawn();
  return createEngine({
    transport,
    fileSource: new BunFileSource(),
    logger: createLogger({ "service.name": "athing-integration-test", "service.version": "0" }),
    hooksSocketPath: HOOKS_SOCK,
    agentHome: agentHome(),
    resolvedCommand: resolveAgentCommand(bashAdapter.binaryResolution),
  });
}

describe("engine integration", () => {
  test(
    "start → ready → onData → kill",
    async () => {
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
    },
    10_000,
  );

  test(
    "two concurrent sessions are isolated",
    async () => {
      const engine = await makeEngine();
      const [s1, s2] = await Promise.all([
        engine.start(bashAdapter, { cwd: process.cwd(), startupTimeoutMs: 5_000 }),
        engine.start(bashAdapter, { cwd: process.cwd(), startupTimeoutMs: 5_000 }),
      ]);

      expect(s1.sessionId).not.toBe(s2.sessionId);

      await Promise.all([s1.kill(), s2.kill()]);
      await engine.shutdown();
    },
    15_000,
  );

  test(
    "BinaryNotFound when binary missing",
    async () => {
      const engine = await makeEngine();
      const badAdapter = {
        ...bashAdapter,
        launch: { ...bashAdapter.launch, command: "__athing_no_such_binary__" },
      };

      const errors: string[] = [];
      const session = await engine.start(badAdapter, {
        cwd: process.cwd(),
        startupTimeoutMs: 5_000,
      });
      session.onError((e) => errors.push(e.kind));

      await new Promise((r) => setTimeout(r, 500));
      expect(errors.some((k) => k === "BinaryNotFound" || k === "SpawnFailed")).toBe(true);

      await engine.shutdown();
    },
    10_000,
  );
});
