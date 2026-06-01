import { test, expect, describe } from "bun:test";
import { spawnSync } from "node:child_process";
import { createEngine } from "@athing/engine";
import { claudeCode } from "@athing/adapter-claude-code";

const hasClaude = (() => {
  const r = spawnSync("which", ["claude"], { encoding: "utf8" });
  return r.status === 0;
})();

describe("engine integration (requires claude binary)", () => {
  test.skipIf(!hasClaude)(
    "start → ready → onData → kill",
    async () => {
      const engine = createEngine();
      const dataChunks: Uint8Array[] = [];

      const session = await engine.start(claudeCode, { startupTimeoutMs: 20_000 });
      session.onData((b) => dataChunks.push(b));

      await new Promise<void>((resolve) => {
        const unsub = session.onStatus((s) => {
          if (s === "IDLE") {
            unsub();
            resolve();
          }
        });
        setTimeout(resolve, 18_000);
      });

      expect(dataChunks.length).toBeGreaterThan(0);

      await session.kill();
      await engine.shutdown();
    },
    25_000,
  );

  test.skipIf(!hasClaude)(
    "two concurrent sessions are isolated",
    async () => {
      const engine = createEngine();
      const [s1, s2] = await Promise.all([
        engine.start(claudeCode, { startupTimeoutMs: 20_000 }),
        engine.start(claudeCode, { startupTimeoutMs: 20_000 }),
      ]);

      expect(s1.sessionId).not.toBe(s2.sessionId);

      await Promise.all([s1.kill(), s2.kill()]);
      await engine.shutdown();
    },
    30_000,
  );

  test.skipIf(!hasClaude)(
    "BinaryNotFound when binary missing",
    async () => {
      const engine = createEngine();
      const badAdapter = {
        ...claudeCode,
        launch: { ...claudeCode.launch, command: "__athing_no_such_binary__" },
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
