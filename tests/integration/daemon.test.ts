import { test, expect, describe, afterAll } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { spawnSync } from "node:child_process";
import { bashAdapter } from "./fixtures/bash-adapter";

const hasDaemon = (() => {
  const r = spawnSync("which", ["athing-daemon"], { encoding: "utf8" });
  return r.status === 0;
})();

const ATHING_DIR = path.join(os.homedir(), ".athing");
const HOOKS_SOCK = path.join(ATHING_DIR, "hooks.sock");
const MANIFEST = path.join(ATHING_DIR, "daemon.json");

describe("daemon survival (requires athing-daemon binary)", () => {
  afterAll(async () => {
    try {
      const raw = fs.readFileSync(MANIFEST, "utf8");
      const { pid } = JSON.parse(raw) as { pid: number };
      process.kill(pid, "SIGTERM");
      await new Promise((r) => setTimeout(r, 300));
    } catch {}
  });

  // 15.5 — daemon spawn + engine reconnect after simulated engine restart
  test.skipIf(!hasDaemon)(
    "session stays alive after engine host restart simulation",
    async () => {
      const { createEngine } = await import("@athing/engine");

      const engine1 = createEngine();
      const session1 = await engine1.start(bashAdapter, {
        startupTimeoutMs: 5_000,
        cwd: os.tmpdir(),
      });
      const sessionId = session1.sessionId;

      await new Promise((r) => setTimeout(r, 200));

      await engine1.shutdown();

      expect(fs.existsSync(MANIFEST)).toBe(true);
      const { pid } = JSON.parse(fs.readFileSync(MANIFEST, "utf8")) as { pid: number };
      let alive = false;
      try {
        process.kill(pid, 0);
        alive = true;
      } catch {}
      expect(alive).toBe(true);

      const engine2 = createEngine();
      const liveIds = await engine2.listSessions();
      expect(liveIds).toContain(sessionId);

      const session2 = await engine2.reconnect(sessionId, bashAdapter, { cwd: os.tmpdir() });
      expect(session2.sessionId).toBe(sessionId);

      await session2.kill();
      await engine2.shutdown();
    },
    30_000,
  );

  // 15.6 — hook delivery continues after engine host restart
  test.skipIf(!hasDaemon)(
    "hook delivery continues after engine restart",
    async () => {
      const { createEngine } = await import("@athing/engine");

      const engine1 = createEngine();
      const session1 = await engine1.start(bashAdapter, {
        startupTimeoutMs: 5_000,
        cwd: os.tmpdir(),
      });

      await new Promise((r) => setTimeout(r, 300));
      await engine1.shutdown();

      expect(fs.existsSync(HOOKS_SOCK)).toBe(true);

      const engine2 = createEngine();
      const session2 = await engine2.reconnect(session1.sessionId, bashAdapter, {
        cwd: os.tmpdir(),
      });

      const statuses: string[] = [];
      session2.onStatus((s) => statuses.push(s));

      await new Promise((r) => setTimeout(r, 500));

      const errors: string[] = [];
      session2.onError((e) => errors.push(e.kind));
      await new Promise((r) => setTimeout(r, 100));
      expect(errors.filter((k) => k === "TransportClosed")).toHaveLength(0);

      await session2.kill();
      await engine2.shutdown();
    },
    30_000,
  );

  // 15.7 — ?id= WS reconnect delivers replay buffer
  test.skipIf(!hasDaemon)(
    "replay buffer delivered on reconnect subscribe",
    async () => {
      const { createEngine } = await import("@athing/engine");

      const engine = createEngine();
      const session1 = await engine.start(bashAdapter, {
        startupTimeoutMs: 5_000,
        cwd: os.tmpdir(),
      });

      const firstChunks: Uint8Array[] = [];
      session1.onData((b) => firstChunks.push(b));

      await new Promise<void>((resolve) => {
        const t = setTimeout(resolve, 5_000);
        session1.onData(() => {
          clearTimeout(t);
          resolve();
        });
      });

      await engine.shutdown();

      const engine2 = createEngine();
      const session2 = await engine2.reconnect(session1.sessionId, bashAdapter, {
        cwd: os.tmpdir(),
      });

      const replayChunks: Uint8Array[] = [];
      session2.onData((b) => replayChunks.push(b));

      await new Promise((r) => setTimeout(r, 200));

      expect(replayChunks.length).toBeGreaterThan(0);

      await session2.kill();
      await engine2.shutdown();
    },
    30_000,
  );
});
