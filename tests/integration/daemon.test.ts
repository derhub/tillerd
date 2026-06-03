import { test, expect, describe } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { encodeFrame, FrameDecoder } from "@athing/sdk";

const ATHING_DIR = process.env["ATHING_DIR"]
  ? path.resolve(process.env["ATHING_DIR"])
  : path.join(os.homedir(), ".athing");

const SOCK_PATH = path.join(ATHING_DIR, "daemon.sock");
const HOOKS_SOCK_PATH = path.join(ATHING_DIR, "hooks.sock");

// ── Socket client ────────────────────────────────────────────────────────────

type Frame = { meta: Record<string, unknown>; body: Buffer | null };

type Client = {
  send(meta: unknown, body?: Buffer): void;
  recv(timeoutMs?: number): Promise<Frame>;
  disconnect(): void;
};

async function connect(): Promise<Client> {
  const decoder = new FrameDecoder();
  const pending: Frame[] = [];
  const waiters: Array<(f: Frame | null) => void> = [];
  let sock: { write(b: Buffer): void; end(): void } | null = null;

  await new Promise<void>((resolve, reject) => {
    Bun.connect({
      unix: SOCK_PATH,
      socket: {
        open(s) {
          sock = s as unknown as { write(b: Buffer): void; end(): void };
          s.write(encodeFrame({ type: "hello", versions: [1] }));
        },
        data(_s, chunk) {
          const buf =
            typeof chunk === "string"
              ? Buffer.from(chunk, "utf8")
              : Buffer.from(chunk as unknown as Uint8Array);
          for (const f of decoder.push(buf)) {
            const meta = f.meta as Record<string, unknown>;
            if (meta["type"] === "hello-ack") {
              resolve();
              continue;
            }
            const frame: Frame = { meta, body: f.body };
            const w = waiters.shift();
            if (w) w(frame);
            else pending.push(frame);
          }
        },
        error(_s, err) {
          reject(err);
        },
        close() {
          for (const w of waiters.splice(0)) w(null);
        },
      },
    });
  });

  return {
    send(meta, body) {
      sock!.write(encodeFrame(meta, body));
    },
    recv(timeoutMs = 5_000) {
      return new Promise((resolve, reject) => {
        const f = pending.shift();
        if (f) {
          resolve(f);
          return;
        }
        const t = setTimeout(() => reject(new Error("recv timeout")), timeoutMs);
        waiters.push((frame) => {
          clearTimeout(t);
          if (frame) resolve(frame);
          else reject(new Error("disconnected"));
        });
      });
    },
    disconnect() {
      sock?.end();
    },
  };
}

async function drainUntil(
  client: Client,
  predicate: (f: Frame) => boolean,
  timeoutMs = 5_000,
): Promise<Frame[]> {
  const collected: Frame[] = [];
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const f = await client.recv(Math.max(100, deadline - Date.now())).catch(() => null);
    if (!f) break;
    collected.push(f);
    if (predicate(f)) break;
  }
  return collected;
}

// ── Session helper ───────────────────────────────────────────────────────────

let sessionCounter = 0;

function nextId(): string {
  return `daemon-test-${process.pid}-${++sessionCounter}`;
}

async function spawnSession(
  client: Client,
  opts: { command: string; args?: string[]; id?: string },
): Promise<{ sessionId: string; pid: number }> {
  const sessionId = opts.id ?? nextId();
  client.send({
    type: "spawn",
    sessionId,
    command: opts.command,
    args: opts.args ?? [],
    flags: [],
    hookSocketPath: HOOKS_SOCK_PATH,
    token: `tok-${sessionId}`,
    cols: 80,
    rows: 24,
    cwd: os.tmpdir(),
  });
  const ack = await client.recv();
  expect(ack.meta["type"]).toBe("spawn-ack");
  return { sessionId, pid: ack.meta["pid"] as number };
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe("daemon protocol", () => {
  test("handshake returns list-ack", async () => {
    const client = await connect();
    client.send({ type: "list" });
    const f = await client.recv();
    expect(f.meta["type"]).toBe("list-ack");
    client.disconnect();
  });

  test("spawn session and receive PTY data", async () => {
    const client = await connect();
    // Use absolute path + simple arg to avoid $SHELL wrapper quoting issues.
    const { sessionId } = await spawnSession(client, {
      command: "/bin/echo",
      args: ["MARKER_DATA"],
    });

    let combined = "";
    const frames = await drainUntil(client, (f) => f.meta["type"] === "exit", 8_000);
    for (const f of frames) {
      if (f.meta["type"] === "data" && f.body) combined += f.body.toString("utf8");
    }

    expect(combined).toContain("MARKER_DATA");
    const exitFrame = frames.find((f) => f.meta["type"] === "exit");
    expect(exitFrame).toBeDefined();
    expect(exitFrame!.meta["sessionId"]).toBe(sessionId);

    client.disconnect();
  }, 15_000);

  test("list returns spawned session ID", async () => {
    const client = await connect();
    const { sessionId } = await spawnSession(client, {
      command: "/bin/sleep",
      args: ["30"],
    });

    client.send({ type: "list" });
    const f = await client.recv();
    expect(f.meta["type"]).toBe("list-ack");
    expect(f.meta["ids"] as string[]).toContain(sessionId);

    client.send({ type: "kill", sessionId });
    client.disconnect();
  }, 10_000);

  test("duplicate spawn returns EEXIST", async () => {
    const client = await connect();
    const id = nextId();
    await spawnSession(client, { command: "/bin/sleep", args: ["30"], id });

    client.send({
      type: "spawn",
      sessionId: id,
      command: "sh",
      args: [],
      flags: [],
      hookSocketPath: HOOKS_SOCK_PATH,
      token: "tok",
      cols: 80,
      rows: 24,
      cwd: os.tmpdir(),
    });
    const err = await client.recv();
    expect(err.meta["type"]).toBe("error");
    expect(err.meta["code"]).toBe("EEXIST");

    client.send({ type: "kill", sessionId: id });
    client.disconnect();
  }, 10_000);

  test("subscribe to unknown session returns ENOTFOUND", async () => {
    const client = await connect();
    client.send({ type: "subscribe", sessionId: "nonexistent-99999" });
    const err = await client.recv();
    expect(err.meta["type"]).toBe("error");
    expect(err.meta["code"]).toBe("ENOTFOUND");
    client.disconnect();
  }, 5_000);

  test("kill delivers exit frame to subscriber", async () => {
    const client = await connect();
    // Spawner is auto-subscribed — kill and drain exit frame directly.
    const { sessionId } = await spawnSession(client, {
      command: "/bin/sleep",
      args: ["60"],
    });

    client.send({ type: "kill", sessionId });

    const frames = await drainUntil(client, (f) => f.meta["type"] === "exit", 5_000);
    const exitFrame = frames.find((f) => f.meta["type"] === "exit");
    expect(exitFrame).toBeDefined();
    expect(exitFrame!.meta["sessionId"]).toBe(sessionId);

    client.disconnect();
  }, 10_000);

  test("continuous PTY output delivered to subscriber", async () => {
    // yes outputs continuously — always in kernel buffer, reliable across PTY read loop impls.
    const client = await connect();
    const { sessionId } = await spawnSession(client, {
      command: "/usr/bin/yes",
      args: ["MARKER_OUTPUT"],
    });

    let combined = "";
    const deadline = Date.now() + 5_000;
    while (!combined.includes("MARKER_OUTPUT") && Date.now() < deadline) {
      const f = await client.recv(1_000).catch(() => null);
      if (!f) continue;
      if (f.meta["type"] === "data" && f.body) {
        combined += f.body.toString("utf8");
        client.send({ type: "ack", sessionId, bytes: f.body.length });
      }
    }

    expect(combined).toContain("MARKER_OUTPUT");
    client.send({ type: "kill", sessionId });
    client.disconnect();
  }, 10_000);

  test("resize does not crash the daemon", async () => {
    const client = await connect();
    const { sessionId } = await spawnSession(client, {
      command: "/bin/sleep",
      args: ["30"],
    });

    client.send({ type: "resize", sessionId, cols: 120, rows: 40 });
    client.send({ type: "list" });
    const f = await client.recv();
    expect(f.meta["type"]).toBe("list-ack");

    client.send({ type: "kill", sessionId });
    client.disconnect();
  }, 10_000);

  test("replay buffer delivered to late subscriber", async () => {
    const clientA = await connect();
    // yes outputs continuously — always buffered, reliably received.
    const { sessionId } = await spawnSession(clientA, {
      command: "/usr/bin/yes",
      args: ["MARKER_REPLAY"],
    });

    let combined = "";
    const deadline = Date.now() + 5_000;
    while (!combined.includes("MARKER_REPLAY") && Date.now() < deadline) {
      const f = await clientA.recv(500).catch(() => null);
      if (!f) continue;
      if (f.meta["type"] === "data" && f.body) {
        combined += f.body.toString("utf8");
        clientA.send({ type: "ack", sessionId, bytes: f.body.length });
      }
    }
    expect(combined).toContain("MARKER_REPLAY");
    clientA.disconnect();

    const clientB = await connect();
    clientB.send({ type: "subscribe", sessionId });

    let replay = "";
    const replayDeadline = Date.now() + 3_000;
    while (!replay.includes("MARKER_REPLAY") && Date.now() < replayDeadline) {
      const f = await clientB.recv(500).catch(() => null);
      if (!f) break;
      if (f.meta["type"] === "data" && f.body) replay += f.body.toString("utf8");
    }
    expect(replay).toContain("MARKER_REPLAY");

    clientB.send({ type: "kill", sessionId });
    clientB.disconnect();
  }, 15_000);

});
