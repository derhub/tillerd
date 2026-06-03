import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { DaemonServer } from "../src/server";
import { encodeFrame, FrameDecoder } from "../src/protocol/codec";
import { parseDaemonFrame } from "../src/protocol/messages";

function tmpDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "athing-snap-test-"));
}

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
          const raw =
            typeof chunk === "string"
              ? Buffer.from(chunk, "utf8")
              : Buffer.from(chunk as unknown as ArrayBuffer);
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

const servers: DaemonServer[] = [];
const dirs: string[] = [];

afterEach(async () => {
  for (const s of servers.splice(0)) await s.shutdown();
  for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
});

describe("capability negotiation via hello handshake", () => {
  test("hello-ack includes capabilities when client advertises snapshot", async () => {
    const dir = tmpDir(); dirs.push(dir);
    const server = new DaemonServer(path.join(dir, "daemon.sock"), path.join(dir, "hooks.sock"));
    await server.start(); servers.push(server);

    const conn = await connectRaw(path.join(dir, "daemon.sock"));
    conn.send({ type: "hello", versions: [1], capabilities: ["snapshot"] });

    const ack = await conn.recv();
    const frame = parseDaemonFrame(ack.meta);
    expect(frame?.type).toBe("hello-ack");
    if (frame?.type === "hello-ack") {
      expect(Array.isArray(frame.capabilities)).toBe(true);
      expect(frame.capabilities).toContain("snapshot");
    }
  });

  test("hello-ack still works when client sends no capabilities (legacy)", async () => {
    const dir = tmpDir(); dirs.push(dir);
    const server = new DaemonServer(path.join(dir, "daemon.sock"), path.join(dir, "hooks.sock"));
    await server.start(); servers.push(server);

    const conn = await connectRaw(path.join(dir, "daemon.sock"));
    conn.send({ type: "hello", versions: [1] });

    const ack = await conn.recv();
    const frame = parseDaemonFrame(ack.meta);
    expect(frame?.type).toBe("hello-ack");
    // Legacy client gets hello-ack, connection not rejected
  });

  test("subscribe to unknown session → error, not snapshot", async () => {
    const dir = tmpDir(); dirs.push(dir);
    const server = new DaemonServer(path.join(dir, "daemon.sock"), path.join(dir, "hooks.sock"));
    await server.start(); servers.push(server);

    const conn = await connectRaw(path.join(dir, "daemon.sock"));
    conn.send({ type: "hello", versions: [1], capabilities: ["snapshot"] });
    await conn.recv(); // hello-ack

    conn.send({ type: "subscribe", sessionId: "nonexistent" });
    const resp = await conn.recv();
    const frame = parseDaemonFrame(resp.meta);
    expect(frame?.type).toBe("error");
  });
});
