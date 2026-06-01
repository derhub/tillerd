/**
 * Integration tests for multi-session isolation and upgrade cycle.
 * These tests require the DaemonServer + real PTY sessions.
 * Tasks 12.1–12.4 (12.3/12.4 also cover task 8.6).
 */
import { test, expect, describe, afterEach } from "bun:test";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { DaemonServer } from "../../packages/daemon/src/server";
import { encodeFrame, FrameDecoder } from "../../packages/daemon/src/protocol/codec";

// Absolute path to the daemon entry point (resolved relative to this file).
const DAEMON_MAIN = path.resolve(import.meta.dir, "../../packages/daemon/src/main.ts");

function createDaemonWrapper(dir: string): string {
  const wrapper = path.join(dir, "athing-daemon");
  fs.writeFileSync(
    wrapper,
    `#!/bin/sh\nexec "${process.execPath}" "${DAEMON_MAIN}" "$@"\n`,
    "utf8",
  );
  fs.chmodSync(wrapper, 0o755);
  return wrapper;
}

async function spawnDaemon(dir: string): Promise<{ pid: number; sockPath: string }> {
  const wrapperPath = createDaemonWrapper(dir);
  const sockPath = path.join(dir, "daemon.sock");
  const manifestPath = path.join(dir, "daemon.json");

  Bun.spawn([wrapperPath], {
    env: { ...process.env, ATHING_DIR: dir, ATHING_DAEMON_BIN: wrapperPath },
    stdio: ["ignore", "ignore", "ignore"],
  }).unref();

  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 100));
    if (fs.existsSync(manifestPath) && fs.existsSync(sockPath)) {
      try {
        const m = JSON.parse(fs.readFileSync(manifestPath, "utf8")) as { pid: number };
        if (m.pid) return { pid: m.pid, sockPath };
      } catch {
        /* manifest not fully written yet */
      }
    }
  }
  throw new Error("Daemon did not start within 10s");
}

async function waitForUpgrade(dir: string, oldPid: number): Promise<{ pid: number }> {
  const manifestPath = path.join(dir, "daemon.json");
  const sockPath = path.join(dir, "daemon.sock");
  const deadline = Date.now() + 20_000;
  while (Date.now() < deadline) {
    await new Promise((r) => setTimeout(r, 200));
    try {
      const m = JSON.parse(fs.readFileSync(manifestPath, "utf8")) as { pid: number };
      if (m.pid && m.pid !== oldPid && fs.existsSync(sockPath)) {
        // Verify socket accepts connections.
        try {
          const c = await connect(sockPath);
          c.disconnect();
          return { pid: m.pid };
        } catch {
          /* not yet */
        }
      }
    } catch {
      /* manifest not readable */
    }
  }
  throw new Error("Daemon upgrade did not complete within 20s");
}

function killDaemon(dir: string): void {
  try {
    const m = JSON.parse(fs.readFileSync(path.join(dir, "daemon.json"), "utf8")) as { pid: number };
    try {
      process.kill(m.pid, "SIGTERM");
    } catch {
      /* already dead */
    }
    setTimeout(() => {
      try {
        process.kill(m.pid, "SIGKILL");
      } catch {
        /* already dead */
      }
    }, 200);
  } catch {
    /* no manifest */
  }
}

const isCI = !!process.env["CI"];

// PTY support requires a real terminal allocation (not available in sandboxed envs).
const hasPty = (() => {
  try {
    const pty = require("node-pty") as typeof import("node-pty");
    const p = pty.spawn("/bin/sh", ["-c", "exit 0"], {
      name: "xterm",
      cols: 80,
      rows: 24,
      cwd: "/tmp",
      env: {},
    });
    p.kill();
    return true;
  } catch {
    return false;
  }
})();

const skip = isCI || !hasPty;

function tmpDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "athing-integ-"));
}

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
              : Buffer.from(chunk as ArrayBuffer);
          for (const f of decoder.push(raw)) {
            if ((f.meta as { type?: string }).type === "hello-ack") {
              resolve();
              continue;
            }
            const w = waiters.shift();
            if (w) {
              w(f);
            } else {
              frames.push(f);
            }
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
    recv(timeoutMs = 5000) {
      return new Promise((resolve, reject) => {
        const f = frames.shift();
        if (f) {
          resolve(f);
          return;
        }
        const timer = setTimeout(() => reject(new Error("recv timeout")), timeoutMs);
        waiters.push((frame) => {
          clearTimeout(timer);
          if (frame) resolve(frame);
          else reject(new Error("disconnected"));
        });
      });
    },
    disconnect() {
      rawSocket?.end();
    },
  };
}

describe("12.1 multi-session: 3 sessions concurrent, independent output", () => {
  const servers: DaemonServer[] = [];
  const dirs: string[] = [];

  afterEach(async () => {
    for (const s of servers.splice(0)) await s.shutdown();
    for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
  });

  test.skipIf(skip)(
    "three sessions each receive their own output",
    async () => {
      const dir = tmpDir();
      dirs.push(dir);
      const server = new DaemonServer(path.join(dir, "d.sock"), path.join(dir, "h.sock"));
      await server.start();
      servers.push(server);

      const clients = await Promise.all([0, 1, 2].map(() => connect(path.join(dir, "d.sock"))));

      // Spawn 3 sessions, each running echo with a unique marker.
      const ids = ["s1", "s2", "s3"];
      const markers = ["MARKER_ONE", "MARKER_TWO", "MARKER_THREE"];

      for (let i = 0; i < 3; i++) {
        clients[i]!.send({
          type: "spawn",
          sessionId: ids[i],
          command: "sh",
          args: ["-c", `echo ${markers[i]!}; sleep 30`],
          flags: [],
          hookSocketPath: path.join(dir, "h.sock"),
          token: `tok${i}`,
          cols: 80,
          rows: 24,
          cwd: os.tmpdir(),
        });
      }

      // Wait for spawn-ack on all sessions.
      for (const client of clients) {
        const ack = await client.recv();
        expect((ack.meta as { type: string }).type).toBe("spawn-ack");
      }

      // Subscribe each client to its own session.
      for (let i = 0; i < 3; i++) {
        clients[i]!.send({ type: "subscribe", sessionId: ids[i] });
      }

      // Collect data from each client and verify marker appears.
      for (let i = 0; i < 3; i++) {
        let combined = "";
        const deadline = Date.now() + 5000;
        while (!combined.includes(markers[i]!) && Date.now() < deadline) {
          try {
            const frame = await clients[i]!.recv(500);
            if ((frame.meta as { type: string }).type === "data" && frame.body) {
              combined += frame.body.toString("utf8");
            }
          } catch {
            break;
          }
        }
        expect(combined).toContain(markers[i]!);
      }

      for (const c of clients) c.disconnect();
    },
    15_000,
  );
});

describe("12.2 flow control: credit exhaustion on one session does not affect other session", () => {
  const servers: DaemonServer[] = [];
  const dirs: string[] = [];

  afterEach(async () => {
    for (const s of servers.splice(0)) await s.shutdown();
    for (const d of dirs.splice(0)) fs.rmSync(d, { recursive: true, force: true });
  });

  test.skipIf(skip)(
    "slow subscriber does not stall fast subscriber on same session",
    async () => {
      const dir = tmpDir();
      dirs.push(dir);
      const server = new DaemonServer(path.join(dir, "d.sock"), path.join(dir, "h.sock"));
      await server.start();
      servers.push(server);

      const clientFast = await connect(path.join(dir, "d.sock"));
      const clientSlow = await connect(path.join(dir, "d.sock"));

      // Spawn one session.
      clientFast.send({
        type: "spawn",
        sessionId: "shared",
        command: "sh",
        args: ["-c", "yes | head -c 200000"],
        flags: [],
        hookSocketPath: path.join(dir, "h.sock"),
        token: "tok",
        cols: 80,
        rows: 24,
        cwd: os.tmpdir(),
      });
      await clientFast.recv(); // spawn-ack

      clientFast.send({ type: "subscribe", sessionId: "shared" });
      clientSlow.send({ type: "subscribe", sessionId: "shared" });

      // Fast client acks; slow client does not.
      let fastBytes = 0;
      const deadline = Date.now() + 4000;
      while (Date.now() < deadline) {
        try {
          const frame = await clientFast.recv(200);
          const meta = frame.meta as { type: string; bodyLen?: number };
          if (meta.type === "data" && frame.body) {
            fastBytes += frame.body.length;
            clientFast.send({ type: "ack", sessionId: "shared", bytes: frame.body.length });
          } else if (meta.type === "exit") break;
        } catch {
          break;
        }
      }

      // Fast client should have received substantial data even though slow client didn't ack.
      expect(fastBytes).toBeGreaterThan(1024);

      clientFast.disconnect();
      clientSlow.disconnect();
    },
    15_000,
  );
});

// ── Process-level upgrade tests (tasks 8.6, 12.3, 12.4) ─────────────────────
// These spawn a real daemon binary so that prepareUpgrade → process.exit(0)
// runs in a separate process rather than the test process.

describe("12.3 / 8.6 upgrade cycle: sessions survive daemon binary upgrade", () => {
  const dirs: string[] = [];

  afterEach(async () => {
    for (const dir of dirs.splice(0)) {
      killDaemon(dir);
      await new Promise((r) => setTimeout(r, 300));
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test.skipIf(skip)(
    "two live sessions survive binary upgrade, output continues after handoff",
    async () => {
      const dir = tmpDir();
      dirs.push(dir);

      const { pid: oldPid, sockPath } = await spawnDaemon(dir);

      // Spawn two sessions that emit continuous output.
      const clients = await Promise.all([connect(sockPath), connect(sockPath)]);
      const ids = ["upgrade-s1", "upgrade-s2"];

      for (let i = 0; i < 2; i++) {
        clients[i]!.send({
          type: "spawn",
          sessionId: ids[i],
          command: "sh",
          args: ["-c", `while true; do echo "tick-${i}"; sleep 0.05; done`],
          flags: [],
          hookSocketPath: path.join(dir, "hooks.sock"),
          token: `tok${i}`,
          cols: 80,
          rows: 24,
          cwd: os.tmpdir(),
        });
        const ack = await clients[i]!.recv();
        expect((ack.meta as { type: string }).type).toBe("spawn-ack");
      }

      // Subscribe both and verify output flows before upgrade.
      for (let i = 0; i < 2; i++) {
        clients[i]!.send({ type: "subscribe", sessionId: ids[i]! });
      }
      for (let i = 0; i < 2; i++) {
        let got = "";
        const deadline = Date.now() + 5000;
        while (!got.includes("tick") && Date.now() < deadline) {
          try {
            const f = await clients[i]!.recv(300);
            if ((f.meta as { type: string }).type === "data" && f.body) {
              got += f.body.toString("utf8");
            }
          } catch {
            break;
          }
        }
        expect(got).toContain("tick");
      }
      for (const c of clients) c.disconnect();

      // Trigger upgrade via a fresh connection.
      {
        const c = await connect(sockPath);
        c.send({ type: "upgrade" });
        c.disconnect();
      }

      // Wait for successor to bind the socket under a new pid.
      const { pid: newPid } = await waitForUpgrade(dir, oldPid);
      expect(newPid).not.toBe(oldPid);

      // Reconnect to successor — both sessions must still be in the registry.
      const postClients = await Promise.all([connect(sockPath), connect(sockPath)]);

      postClients[0]!.send({ type: "list" });
      const listAck = await postClients[0]!.recv();
      const liveIds = (listAck.meta as { ids: string[] }).ids;
      expect(liveIds).toContain(ids[0]);
      expect(liveIds).toContain(ids[1]);

      // Subscribe and confirm output still flows from the adopted sessions.
      for (let i = 0; i < 2; i++) {
        postClients[i]!.send({ type: "subscribe", sessionId: ids[i]! });
      }
      for (let i = 0; i < 2; i++) {
        let got = "";
        const deadline = Date.now() + 8000;
        while (!got.includes("tick") && Date.now() < deadline) {
          try {
            const f = await postClients[i]!.recv(300);
            if ((f.meta as { type: string }).type === "data" && f.body) {
              got += f.body.toString("utf8");
            }
          } catch {
            break;
          }
        }
        expect(got).toContain("tick");
      }
      for (const c of postClients) c.disconnect();
    },
    45_000,
  );
});

describe("12.4 reconnect after upgrade: replay buffer intact", () => {
  const dirs: string[] = [];

  afterEach(async () => {
    for (const dir of dirs.splice(0)) {
      killDaemon(dir);
      await new Promise((r) => setTimeout(r, 300));
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test.skipIf(skip)(
    "client reconnects after upgrade and receives pre-upgrade replay buffer",
    async () => {
      const dir = tmpDir();
      dirs.push(dir);

      const { pid: oldPid, sockPath } = await spawnDaemon(dir);

      // Spawn a session that emits a unique marker then sleeps.
      const client = await connect(sockPath);
      client.send({
        type: "spawn",
        sessionId: "replay-session",
        command: "sh",
        args: ["-c", "echo BEFORE_UPGRADE; sleep 60"],
        flags: [],
        hookSocketPath: path.join(dir, "hooks.sock"),
        token: "tok",
        cols: 80,
        rows: 24,
        cwd: os.tmpdir(),
      });
      await client.recv(); // spawn-ack

      // Subscribe and wait for the marker to arrive in the replay buffer.
      client.send({ type: "subscribe", sessionId: "replay-session" });
      let preData = "";
      const preDeadline = Date.now() + 5000;
      while (!preData.includes("BEFORE_UPGRADE") && Date.now() < preDeadline) {
        try {
          const f = await client.recv(300);
          const meta = f.meta as { type: string };
          if (meta.type === "data" && f.body) {
            preData += f.body.toString("utf8");
            client.send({ type: "ack", sessionId: "replay-session", bytes: f.body.length });
          }
        } catch {
          break;
        }
      }
      expect(preData).toContain("BEFORE_UPGRADE");

      // Disconnect — session stays alive in daemon.
      client.disconnect();

      // Trigger upgrade.
      {
        const c = await connect(sockPath);
        c.send({ type: "upgrade" });
        c.disconnect();
      }

      const { pid: newPid } = await waitForUpgrade(dir, oldPid);
      expect(newPid).not.toBe(oldPid);

      // Reconnect to successor and subscribe — replay buffer must include the
      // marker that was written before the upgrade.
      const newClient = await connect(sockPath);
      newClient.send({ type: "subscribe", sessionId: "replay-session" });

      let replayData = "";
      const replayDeadline = Date.now() + 5000;
      while (!replayData.includes("BEFORE_UPGRADE") && Date.now() < replayDeadline) {
        try {
          const f = await newClient.recv(300);
          const meta = f.meta as { type: string };
          if (meta.type === "data" && f.body) replayData += f.body.toString("utf8");
        } catch {
          break;
        }
      }
      expect(replayData).toContain("BEFORE_UPGRADE");

      newClient.disconnect();
    },
    40_000,
  );
});
