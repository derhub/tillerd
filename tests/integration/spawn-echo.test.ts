/**
 * Focused integration test: spawn a PTY session via DaemonServer, run
 * `echo "success"`, and verify the word "success" appears in the output.
 *
 * This test imports DaemonServer directly from packages/daemon so that
 * node-pty is resolved from packages/daemon/node_modules (not the integration
 * test workspace, where node-pty is not installed).
 */
import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { DaemonServer } from "../../packages/daemon/src/server";
import { encodeFrame, FrameDecoder } from "../../packages/daemon/src/protocol/codec";

// ── helpers (copied from multi-session.test.ts) ──────────────────────────────

type Client = {
  send(meta: unknown, body?: Buffer): void;
  recv(timeoutMs?: number): Promise<{ meta: unknown; body: Buffer | null }>;
  disconnect(): void;
};

async function connect(sockPath: string): Promise<Client> {
  const decoder = new FrameDecoder();
  const frames: Array<{ meta: unknown; body: Buffer | null }> = [];
  const waiters: Array<(f: { meta: unknown; body: Buffer | null } | null) => void> = [];
  let rawSocket: { write(b: Buffer): void; end(): void } | null = null;

  await new Promise<void>((resolve, reject) => {
    Bun.connect({
      unix: sockPath,
      socket: {
        open: (s) => {
          rawSocket = s as unknown as { write(b: Buffer): void; end(): void };
          s.write(encodeFrame({ type: "hello", versions: [1] }));
        },
        data: (_s, chunk) => {
          const raw =
            typeof chunk === "string"
              ? Buffer.from(chunk, "utf8")
              : Buffer.from(chunk as unknown as Uint8Array);
          for (const f of decoder.push(raw)) {
            if ((f.meta as { type?: string }).type === "hello-ack") {
              resolve();
              continue;
            }
            const w = waiters.shift();
            if (w) w(f);
            else frames.push(f);
          }
        },
        error: (_s, err) => reject(err),
        close: () => {
          for (const w of waiters.splice(0)) w(null);
        },
      },
    });
  });

  return {
    send(meta, body) {
      rawSocket!.write(encodeFrame(meta, body));
    },
    recv(timeoutMs = 8000) {
      return new Promise((resolve, reject) => {
        const f = frames.shift();
        if (f) {
          resolve(f);
          return;
        }
        const timer = setTimeout(
          () => reject(new Error("recv timeout after " + timeoutMs + "ms")),
          timeoutMs,
        );
        waiters.push((frame) => {
          clearTimeout(timer);
          if (frame) resolve(frame);
          else reject(new Error("socket disconnected"));
        });
      });
    },
    disconnect() {
      rawSocket?.end();
    },
  };
}

// ── test ─────────────────────────────────────────────────────────────────────

describe("daemon: spawn terminal with echo success", () => {
  const servers: DaemonServer[] = [];
  const dirs: string[] = [];

  afterEach(async () => {
    for (const s of servers.splice(0)) {
      try {
        await s.shutdown();
      } catch {
        /* ignore */
      }
    }
    for (const d of dirs.splice(0)) {
      try {
        fs.rmSync(d, { recursive: true, force: true });
      } catch {
        /* ignore */
      }
    }
  });

  test('spawn `sh -c "echo success"` and receive "success" in PTY output', async () => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "athing-echo-"));
    dirs.push(dir);

    const sockPath = path.join(dir, "daemon.sock");
    const hooksSockPath = path.join(dir, "hooks.sock");

    const server = new DaemonServer(sockPath, hooksSockPath);
    await server.start();
    servers.push(server);

    const client = await connect(sockPath);

    // Spawn a PTY session running `echo success`.
    // PtyTransport wraps args with `$SHELL -lc "exec <cmd> <args>"`.
    // Use absolute /bin/echo to avoid shell-builtin resolution on zsh.
    client.send({
      type: "spawn",
      sessionId: "echo-test",
      command: "/bin/echo",
      args: ["success"],
      flags: [],
      hookSocketPath: hooksSockPath,
      token: "test-token",
      cols: 80,
      rows: 24,
      cwd: os.tmpdir(),
    });

    // Wait for spawn-ack.
    const ack = await client.recv(8000);
    expect((ack.meta as { type: string }).type).toBe("spawn-ack");

    // Subscribe to session output.
    client.send({ type: "subscribe", sessionId: "echo-test" });

    // Collect output until "success" appears or timeout.
    let combined = "";
    const deadline = Date.now() + 8000;
    while (!combined.includes("success") && Date.now() < deadline) {
      try {
        const frame = await client.recv(1000);
        const meta = frame.meta as { type: string };
        if (meta.type === "data" && frame.body) {
          combined += frame.body.toString("utf8");
        } else if (meta.type === "exit") {
          break; // process finished
        }
      } catch {
        break; // timeout on recv — stop waiting
      }
    }

    client.disconnect();

    expect(combined).toContain("success");
  }, 15_000);
});
