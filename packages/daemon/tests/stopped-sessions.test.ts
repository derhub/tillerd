import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { StoppedSessionsStore } from "../src/stopped-sessions";
import { DaemonServer } from "../src/server";
import { encodeFrame, FrameDecoder } from "../src/protocol/codec";
import { parseDaemonFrame } from "../src/protocol/messages";

function tmpDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "athing-stop-test-"));
}

const dirs: string[] = [];
const servers: DaemonServer[] = [];

afterEach(async () => {
  for (const s of servers.splice(0)) await s.shutdown();
  for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
});

describe("StoppedSessionsStore — unit", () => {
  test("has() returns false for unknown session", () => {
    const dir = tmpDir(); dirs.push(dir);
    const store = new StoppedSessionsStore(path.join(dir, "stopped.txt"));
    store.load();
    expect(store.has("unknown")).toBe(false);
  });

  test("add() → has() returns true", () => {
    const dir = tmpDir(); dirs.push(dir);
    const store = new StoppedSessionsStore(path.join(dir, "stopped.txt"));
    store.load();
    store.add("session-abc");
    expect(store.has("session-abc")).toBe(true);
  });

  test("persists across load() (durable)", () => {
    const dir = tmpDir(); dirs.push(dir);
    const filePath = path.join(dir, "stopped.txt");
    const store1 = new StoppedSessionsStore(filePath);
    store1.load();
    store1.add("durable-id");

    const store2 = new StoppedSessionsStore(filePath);
    store2.load();
    expect(store2.has("durable-id")).toBe(true);
  });

  test("in-memory cache eviction does not resurrect durably stopped sessions", () => {
    const dir = tmpDir(); dirs.push(dir);
    const filePath = path.join(dir, "stopped.txt");
    const store = new StoppedSessionsStore(filePath);
    store.load();
    store.add("persistent-id");

    // Simulate cache eviction by reloading from disk (fresh instance)
    const freshStore = new StoppedSessionsStore(filePath);
    freshStore.load();
    // Durable record must survive
    expect(freshStore.has("persistent-id")).toBe(true);
  });
});

async function connectRaw(sockPath: string): Promise<{
  send(meta: unknown, body?: Buffer): void;
  recv(): Promise<{ meta: unknown; body: Buffer | null }>;
}> {
  const decoder = new FrameDecoder();
  const frames: Array<{ meta: unknown; body: Buffer | null }> = [];
  const waiters: Array<(f: { meta: unknown; body: Buffer | null }) => void> = [];

  return new Promise((resolve, reject) => {
    Bun.connect({
      unix: sockPath,
      socket: {
        open: (s) => {
          const send = (meta: unknown, body?: Buffer) =>
            (s as unknown as { write(b: Buffer): void }).write(encodeFrame(meta, body));
          const recv = () =>
            new Promise<{ meta: unknown; body: Buffer | null }>((res) => {
              const f = frames.shift();
              if (f) res(f);
              else waiters.push(res);
            });
          resolve({ send, recv });
        },
        data: (_s, chunk) => {
          const raw = typeof chunk === "string" ? Buffer.from(chunk, "utf8") : Buffer.from(chunk as unknown as ArrayBuffer);
          for (const f of decoder.push(raw)) {
            const waiter = waiters.shift();
            if (waiter) waiter(f);
            else frames.push(f);
          }
        },
        error: (_s, err) => reject(err),
        close: () => {},
      },
    });
  });
}

async function handshake(sockPath: string) {
  const conn = await connectRaw(sockPath);
  conn.send({ type: "hello", versions: [1] });
  await conn.recv(); // hello-ack
  return conn;
}

describe("DaemonServer — stop frame + spawn resume guard", () => {
  test("stop frame → subsequent spawn with resume rejected as SessionStopped", async () => {
    const dir = tmpDir(); dirs.push(dir);
    const stoppedPath = path.join(dir, "stopped.txt");
    const server = new DaemonServer(path.join(dir, "daemon.sock"), path.join(dir, "hooks.sock"), stoppedPath);
    await server.start(); servers.push(server);

    const conn = await handshake(path.join(dir, "daemon.sock"));

    // Send stop for a session that doesn't exist (no PTY) — stop just adds to store
    conn.send({ type: "stop", sessionId: "old-session-id" });
    await new Promise((r) => setTimeout(r, 50)); // allow async handler

    // Now try spawn with resume pointing to the stopped session
    conn.send({
      type: "spawn",
      sessionId: "new-session-id",
      resume: "old-session-id",
      command: "/bin/true",
      args: [],
      flags: [],
      hookSocketPath: path.join(dir, "hooks.sock"),
      token: "tok",
      cols: 80,
      rows: 24,
      cwd: dir,
    });

    const resp = await conn.recv();
    const frame = parseDaemonFrame(resp.meta);
    expect(frame?.type).toBe("error");
    if (frame?.type === "error") {
      expect(frame.code).toBe("SessionStopped");
    }
  });

  test("stop state durable across daemon restart", async () => {
    const dir = tmpDir(); dirs.push(dir);
    const stoppedPath = path.join(dir, "stopped.txt");

    // First daemon instance: stop a session
    const server1 = new DaemonServer(path.join(dir, "daemon.sock"), path.join(dir, "hooks.sock"), stoppedPath);
    await server1.start();
    const conn1 = await handshake(path.join(dir, "daemon.sock"));
    conn1.send({ type: "stop", sessionId: "durable-session" });
    await new Promise((r) => setTimeout(r, 50));
    await server1.shutdown();
    await new Promise((r) => setTimeout(r, 50));

    // Second daemon instance: same stopped-sessions file
    const server2 = new DaemonServer(path.join(dir, "daemon2.sock"), path.join(dir, "hooks2.sock"), stoppedPath);
    await server2.start(); servers.push(server2);
    const conn2 = await handshake(path.join(dir, "daemon2.sock"));

    conn2.send({
      type: "spawn",
      sessionId: "new-id",
      resume: "durable-session",
      command: "/bin/true",
      args: [], flags: [],
      hookSocketPath: path.join(dir, "hooks2.sock"),
      token: "tok", cols: 80, rows: 24, cwd: dir,
    });

    const resp = await conn2.recv();
    const frame = parseDaemonFrame(resp.meta);
    expect(frame?.type).toBe("error");
    if (frame?.type === "error") {
      expect(frame.code).toBe("SessionStopped");
    }
  });

  test("kill (not stop) does not block subsequent resume", async () => {
    const dir = tmpDir(); dirs.push(dir);
    const stoppedPath = path.join(dir, "stopped.txt");
    const server = new DaemonServer(path.join(dir, "daemon.sock"), path.join(dir, "hooks.sock"), stoppedPath);
    await server.start(); servers.push(server);

    const conn = await handshake(path.join(dir, "daemon.sock"));

    // kill does NOT add to stopped set
    conn.send({ type: "kill", sessionId: "killed-session" });
    await new Promise((r) => setTimeout(r, 50));

    // Spawn with resume pointing to killed (not stopped) session should NOT be blocked
    conn.send({
      type: "spawn",
      sessionId: "recovery-id",
      resume: "killed-session",
      command: "/bin/true",
      args: [], flags: [],
      hookSocketPath: path.join(dir, "hooks.sock"),
      token: "tok", cols: 80, rows: 24, cwd: dir,
    });

    const resp = await conn.recv();
    const frame = parseDaemonFrame(resp.meta);
    // Should NOT be SessionStopped — may be spawn-ack or SpawnFailed (binary env), but not stop-blocked
    if (frame?.type === "error") {
      expect((frame as { code: string }).code).not.toBe("SessionStopped");
    }
  });
});
