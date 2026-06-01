import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { DaemonServer } from "../src/server";
import { encodeFrame, FrameDecoder } from "../src/protocol/codec";

function tmpDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "athing-handshake-"));
}

async function connectRaw(sockPath: string): Promise<{
  send(meta: unknown, body?: Buffer): void;
  recv(): Promise<{ meta: unknown; body: Buffer | null }>;
}> {
  const decoder = new FrameDecoder();
  const frames: Array<{ meta: unknown; body: Buffer | null }> = [];
  const waiters: Array<(f: { meta: unknown; body: Buffer | null }) => void> = [];

  return new Promise((resolve, reject) => {
    const sock = Bun.connect({
      unix: sockPath,
      socket: {
        open: (s) => {
          const send = (meta: unknown, body?: Buffer) => s.write(encodeFrame(meta, body));
          const recv = () =>
            new Promise<{ meta: unknown; body: Buffer | null }>((res) => {
              const f = frames.shift();
              if (f) {
                res(f);
              } else {
                waiters.push(res);
              }
            });
          resolve({ send, recv });
        },
        data: (_s, chunk) => {
          const raw =
            typeof chunk === "string"
              ? Buffer.from(chunk, "utf8")
              : Buffer.from(chunk as ArrayBuffer);
          for (const f of decoder.push(raw)) {
            const waiter = waiters.shift();
            if (waiter) {
              waiter(f);
            } else {
              frames.push(f);
            }
          }
        },
        error: (_s, err) => reject(err),
        close: () => {},
      },
    });
    void sock;
  });
}

async function startServer(dir: string): Promise<DaemonServer> {
  const sockPath = path.join(dir, "daemon.sock");
  const hooksSockPath = path.join(dir, "hooks.sock");
  const server = new DaemonServer(sockPath, hooksSockPath);
  await server.start();
  return server;
}

describe("protocol handshake", () => {
  const servers: DaemonServer[] = [];
  const dirs: string[] = [];

  afterEach(async () => {
    for (const s of servers.splice(0)) await s.shutdown();
    for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
  });

  test("compatible handshake completes", async () => {
    const dir = tmpDir();
    dirs.push(dir);
    const server = await startServer(dir);
    servers.push(server);

    const conn = await connectRaw(path.join(dir, "daemon.sock"));
    conn.send({ type: "hello", versions: [1] });

    const frame = await conn.recv();
    expect((frame.meta as { type: string }).type).toBe("hello-ack");
    expect((frame.meta as { version: number }).version).toBe(1);
  });

  test("incompatible version closes connection with error frame", async () => {
    const dir = tmpDir();
    dirs.push(dir);
    const server = await startServer(dir);
    servers.push(server);

    const conn = await connectRaw(path.join(dir, "daemon.sock"));
    conn.send({ type: "hello", versions: [99999] });

    const frame = await conn.recv();
    expect((frame.meta as { type: string }).type).toBe("error");
    expect((frame.meta as { code: string }).code).toBe("EVERSION");
  });

  test("message before handshake rejected with EPROTO", async () => {
    const dir = tmpDir();
    dirs.push(dir);
    const server = await startServer(dir);
    servers.push(server);

    const conn = await connectRaw(path.join(dir, "daemon.sock"));
    conn.send({ type: "list" }); // skip hello

    const frame = await conn.recv();
    expect((frame.meta as { type: string }).type).toBe("error");
    expect((frame.meta as { code: string }).code).toBe("EPROTO");
  });
});
