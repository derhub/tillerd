import { test, expect, describe } from "bun:test";
import { createEngine } from "@athing/engine";
import { bashAdapter } from "./fixtures/bash-adapter";

describe("engine integration", () => {
  test(
    "start → ready → onData → kill",
    async () => {
      const engine = createEngine();
      const dataChunks: Uint8Array[] = [];

      const session = await engine.start(bashAdapter, { startupTimeoutMs: 5_000 });
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
      const engine = createEngine();
      const [s1, s2] = await Promise.all([
        engine.start(bashAdapter, { startupTimeoutMs: 5_000 }),
        engine.start(bashAdapter, { startupTimeoutMs: 5_000 }),
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
      const engine = createEngine();
      const badAdapter = {
        ...bashAdapter,
        launch: { ...bashAdapter.launch, command: "__athing_no_such_binary__" },
      };

      const errors: string[] = [];
      const session = await engine.start(badAdapter, { startupTimeoutMs: 5_000 });
      session.onError((e) => errors.push(e.kind));

      await new Promise((r) => setTimeout(r, 500));
      expect(errors.some((k) => k === "BinaryNotFound" || k === "SpawnFailed")).toBe(true);

      await engine.shutdown();
    },
    10_000,
  );
});
