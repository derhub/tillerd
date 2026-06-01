import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { DaemonServer } from "../src/server";
import { encodeFrame, FrameDecoder } from "../src/protocol/codec";

function tmpDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "athing-upgrade-"));
}

async function startServer(dir: string): Promise<DaemonServer> {
  const sockPath = path.join(dir, "daemon.sock");
  const hooksSockPath = path.join(dir, "hooks.sock");
  const server = new DaemonServer(sockPath, hooksSockPath);
  await server.start();
  return server;
}

async function connectAndHandshake(sockPath: string) {
  const decoder = new FrameDecoder();
  const frames: Array<{ meta: unknown; body: Buffer | null }> = [];
  const waiters: Array<(f: { meta: unknown; body: Buffer | null }) => void> = [];
  let socket_: ReturnType<typeof Bun.connect> | null = null;

  await new Promise<void>((resolve, reject) => {
    Bun.connect({
      unix: sockPath,
      socket: {
        open: (s) => {
          socket_ = s as unknown as ReturnType<typeof Bun.connect>;
          s.write(encodeFrame({ type: "hello", versions: [1] }));
        },
        data: (_s, chunk) => {
          const raw =
            typeof chunk === "string"
              ? Buffer.from(chunk, "utf8")
              : Buffer.from(chunk as ArrayBuffer);
          for (const f of decoder.push(raw)) {
            if (f && (f.meta as { type?: string }).type === "hello-ack") {
              resolve();
              continue;
            }
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
  });

  return {
    send(meta: unknown, body?: Buffer) {
      (socket_ as unknown as { write(b: Buffer): void }).write(encodeFrame(meta, body));
    },
    recv(): Promise<{ meta: unknown; body: Buffer | null }> {
      return new Promise((res) => {
        const f = frames.shift();
        if (f) {
          res(f);
        } else {
          waiters.push(res);
        }
      });
    },
  };
}

describe("9.5: upgrade frame dispatched via wire protocol, server continues serving", () => {
  const servers: DaemonServer[] = [];
  const dirs: string[] = [];

  afterEach(async () => {
    for (const s of servers.splice(0)) await s.shutdown();
    for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
  });

  test("upgrade frame triggers prepareUpgrade; server remains reachable when binary absent", async () => {
    const dir = tmpDir();
    dirs.push(dir);
    const server = await startServer(dir);
    servers.push(server);

    const prev = process.env["ATHING_DAEMON_BIN"];
    process.env["ATHING_DAEMON_BIN"] = "/nonexistent/athing-daemon";

    try {
      const conn = await connectAndHandshake(path.join(dir, "daemon.sock"));
      conn.send({ type: "upgrade" });

      // Give prepareUpgrade time to attempt and fail gracefully.
      await new Promise((r) => setTimeout(r, 300));

      // Server must still respond — predecessor survived.
      conn.send({ type: "list" });
      const frame = await conn.recv();
      expect((frame.meta as { type: string }).type).toBe("list-ack");
    } finally {
      if (prev === undefined) delete process.env["ATHING_DAEMON_BIN"];
      else process.env["ATHING_DAEMON_BIN"] = prev;
    }
  }, 5_000);
});

describe("upgrade: nak causes predecessor to survive and continue serving", () => {
  const servers: DaemonServer[] = [];
  const dirs: string[] = [];

  afterEach(async () => {
    for (const s of servers.splice(0)) await s.shutdown();
    for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
  });

  test("prepareUpgrade with missing daemon binary logs and returns without exiting", async () => {
    const dir = tmpDir();
    dirs.push(dir);
    const server = await startServer(dir);
    servers.push(server);

    // Override ATHING_DAEMON_BIN to a non-existent path so spawn fails.
    const prev = process.env["ATHING_DAEMON_BIN"];
    process.env["ATHING_DAEMON_BIN"] = "/nonexistent/athing-daemon";

    let threw = false;
    try {
      await (server as unknown as { prepareUpgrade(): Promise<void> }).prepareUpgrade();
    } catch {
      threw = true;
    } finally {
      if (prev === undefined) {
        delete process.env["ATHING_DAEMON_BIN"];
      } else {
        process.env["ATHING_DAEMON_BIN"] = prev;
      }
    }

    // Should not throw — prepareUpgrade catches errors internally.
    expect(threw).toBe(false);

    // Server should still be accepting connections (predecessor survived).
    const conn = await connectAndHandshake(path.join(dir, "daemon.sock"));
    conn.send({ type: "list" });
    const frame = await conn.recv();
    expect((frame.meta as { type: string }).type).toBe("list-ack");
  });
});
