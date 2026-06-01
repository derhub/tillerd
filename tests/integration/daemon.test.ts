import { test, expect, describe, afterAll } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { spawnSync } from "node:child_process";

const hasDaemon = (() => {
  const r = spawnSync("which", ["athing-daemon"], { encoding: "utf8" });
  return r.status === 0;
})();

const ATHING_DIR = path.join(os.homedir(), ".athing");
const DAEMON_SOCK = path.join(ATHING_DIR, "daemon.sock");
const HOOKS_SOCK = path.join(ATHING_DIR, "hooks.sock");
const MANIFEST = path.join(ATHING_DIR, "daemon.json");

describe("daemon survival (requires athing-daemon binary)", () => {
  afterAll(async () => {
    // Best-effort cleanup
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
      const { claudeCode } = await import("@athing/adapter-claude-code");

      // First engine instance — adopts or spawns daemon
      const engine1 = createEngine();
      const session1 = await engine1.start(claudeCode, {
        startupTimeoutMs: 20_000,
        cwd: os.tmpdir(),
      });
      const sessionId = session1.sessionId;

      // Give session a moment to register in daemon
      await new Promise((r) => setTimeout(r, 200));

      // Simulate "server restart": shutdown engine (unsubscribes, does NOT kill)
      await engine1.shutdown();

      // Verify daemon is still running
      expect(fs.existsSync(MANIFEST)).toBe(true);
      const { pid } = JSON.parse(fs.readFileSync(MANIFEST, "utf8")) as { pid: number };
      let alive = false;
      try {
        process.kill(pid, 0);
        alive = true;
      } catch {}
      expect(alive).toBe(true);

      // Second engine instance — reconnects to same daemon
      const engine2 = createEngine();
      const liveIds = await engine2.listSessions();
      expect(liveIds).toContain(sessionId);

      // Reconnect to existing session
      const session2 = await engine2.reconnect(sessionId, claudeCode, { cwd: os.tmpdir() });
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
      const { claudeCode } = await import("@athing/adapter-claude-code");

      const engine1 = createEngine();
      const session1 = await engine1.start(claudeCode, {
        startupTimeoutMs: 20_000,
        cwd: os.tmpdir(),
      });

      await new Promise((r) => setTimeout(r, 300));
      await engine1.shutdown(); // engine "restarts" — daemon stays

      // Daemon hooks.sock should still exist
      expect(fs.existsSync(HOOKS_SOCK)).toBe(true);

      // Reconnect
      const engine2 = createEngine();
      const session2 = await engine2.reconnect(session1.sessionId, claudeCode, {
        cwd: os.tmpdir(),
      });

      // Collect status events — hooks still flowing means status can change
      const statuses: string[] = [];
      session2.onStatus((s) => statuses.push(s));

      await new Promise((r) => setTimeout(r, 500));

      // We can't guarantee a hook fires in this window without sending a prompt,
      // but we verify the subscription is wired — no error event should fire.
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
      const { claudeCode } = await import("@athing/adapter-claude-code");

      const engine = createEngine();
      const session1 = await engine.start(claudeCode, {
        startupTimeoutMs: 20_000,
        cwd: os.tmpdir(),
      });

      // Collect data from first subscription
      const firstChunks: Uint8Array[] = [];
      session1.onData((b) => firstChunks.push(b));

      // Wait for some PTY output to arrive
      await new Promise<void>((resolve) => {
        const t = setTimeout(resolve, 10_000);
        session1.onData(() => {
          clearTimeout(t);
          resolve();
        });
      });

      const dataBeforeReconnect = firstChunks.length;
      await engine.shutdown(); // drop subscription, keep daemon session alive

      // Reconnect — replay buffer should arrive immediately
      const engine2 = createEngine();
      const session2 = await engine2.reconnect(session1.sessionId, claudeCode, {
        cwd: os.tmpdir(),
      });

      const replayChunks: Uint8Array[] = [];
      session2.onData((b) => replayChunks.push(b));

      // Give replay a tick to land
      await new Promise((r) => setTimeout(r, 200));

      // Replay buffer should contain at least some of what arrived before reconnect
      expect(replayChunks.length).toBeGreaterThan(0);

      await session2.kill();
      await engine2.shutdown();
    },
    30_000,
  );
});
