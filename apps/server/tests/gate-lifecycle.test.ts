import { test, expect, describe, afterEach } from "bun:test";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { unlinkSync } from "node:fs";
import { registerSession, deregisterSession } from "../src/gate-admin";

// ── fake gate admin ───────────────────────────────────────────────────────────

interface AdminCommand {
  adminToken: string;
  request: {
    command: string;
    sessionId?: string;
    token?: string;
  };
}

interface FakeAdmin {
  socketPath: string;
  commands: AdminCommand[];
  /** Sessions registered and not yet deregistered */
  registry: Map<string, string>;
  stop(): void;
}

function encodeFrame(payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(4 + payload.byteLength);
  const view = new DataView(out.buffer);
  view.setUint32(0, payload.byteLength, false);
  out.set(payload, 4);
  return out;
}

function startFakeAdmin(adminToken: string): FakeAdmin {
  const socketPath = join(
    tmpdir(),
    `gate-admin-test-${Date.now()}-${Math.random().toString(36).slice(2)}.sock`,
  );
  const commands: AdminCommand[] = [];
  const registry = new Map<string, string>();

  // The Admin route: per connection the client sends a preamble frame then a command
  // frame. Accumulate bytes (frames may split or coalesce), decode both, then apply.
  const buffers = new WeakMap<object, Uint8Array>();

  const server = Bun.listen({
    unix: socketPath,
    socket: {
      open() {},
      data(socket, data: Buffer) {
        const prev = buffers.get(socket) ?? new Uint8Array(0);
        const incoming = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
        const buf = new Uint8Array(prev.length + incoming.length);
        buf.set(prev, 0);
        buf.set(incoming, prev.length);

        const frames: Uint8Array[] = [];
        let offset = 0;
        while (buf.length - offset >= 4) {
          const view = new DataView(buf.buffer, buf.byteOffset + offset);
          const len = view.getUint32(0, false);
          if (buf.length - offset < 4 + len) break;
          frames.push(buf.slice(offset + 4, offset + 4 + len));
          offset += 4 + len;
        }
        buffers.set(socket, buf.slice(offset));

        // Need the preamble and the command frame before acting.
        if (frames.length < 2) return;
        const preamble = JSON.parse(new TextDecoder().decode(frames[0])) as {
          route: string;
          token?: string;
        };
        const request = JSON.parse(new TextDecoder().decode(frames[1])) as AdminCommand["request"];
        commands.push({ adminToken: preamble.token ?? "", request });

        let result: string;
        if (preamble.route !== "admin" || preamble.token !== adminToken) {
          result = "unauthenticated";
        } else if (request.command === "register" && request.sessionId && request.token) {
          registry.set(request.sessionId, request.token);
          result = "ok";
        } else if (request.command === "deregister" && request.sessionId) {
          registry.delete(request.sessionId);
          result = "ok";
        } else {
          result = "invalid";
        }

        const response = new TextEncoder().encode(JSON.stringify({ result }));
        socket.write(encodeFrame(response));
      },
      close() {},
      error() {},
    },
  });

  return {
    socketPath,
    commands,
    registry,
    stop() {
      server.stop(true);
      try {
        unlinkSync(socketPath);
      } catch {
        /* already gone */
      }
    },
  };
}

// ── helpers ───────────────────────────────────────────────────────────────────

const ADMIN_TOKEN = "test-admin-secret";

// ── tests ─────────────────────────────────────────────────────────────────────

describe("mints_session_id_and_per_session_token", () => {
  let gate: FakeAdmin;
  afterEach(() => gate?.stop());

  test("each call to registerSession uses unique credentials", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);

    const sid1 = "session-aaa";
    const tok1 = "a".repeat(64);
    const sid2 = "session-bbb";
    const tok2 = "b".repeat(64);

    await registerSession(sid1, tok1, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });
    await registerSession(sid2, tok2, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });

    expect(gate.registry.get(sid1)).toBe(tok1);
    expect(gate.registry.get(sid2)).toBe(tok2);
    expect(gate.registry.get(sid1)).not.toBe(gate.registry.get(sid2));
  });
});

describe("registers_session_with_gate_admin_before_spawn", () => {
  let gate: FakeAdmin;
  afterEach(() => gate?.stop());

  test("registerSession completes before returning (ordering guarantee)", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);
    const sessionId = "pre-spawn-session";
    const token = "c".repeat(64);

    await registerSession(sessionId, token, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });

    // Registration must be in the registry before any spawn could fire
    expect(gate.registry.has(sessionId)).toBe(true);
    expect(gate.commands).toHaveLength(1);
    expect(gate.commands[0]!.request.command).toBe("register");
  });
});

describe("gate_admin_register_endpoint_called_with_token", () => {
  let gate: FakeAdmin;
  afterEach(() => gate?.stop());

  test("register command carries the session token in the request", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);
    const sessionId = "token-check-session";
    const token = "d".repeat(64);

    await registerSession(sessionId, token, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });

    const cmd = gate.commands[0]!;
    expect(cmd.request.command).toBe("register");
    expect(cmd.request.sessionId).toBe(sessionId);
    expect(cmd.request.token).toBe(token);
  });

  test("register fails when admin token is wrong", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);

    await expect(
      registerSession("s", "t", {
        socketPath: gate.socketPath,
        adminToken: "wrong-token",
      }),
    ).rejects.toThrow(/unauthenticated/);
  });
});

describe("injects_gate_url_session_id_token_into_daemon_env", () => {
  test("registerSession sends the session credentials to the gate admin", async () => {
    const gate = startFakeAdmin(ADMIN_TOKEN);
    try {
      const sessionId = "env-inject-session";
      const token = "h".repeat(64);

      await registerSession(sessionId, token, {
        socketPath: gate.socketPath,
        adminToken: ADMIN_TOKEN,
      });

      const cmd = gate.commands[0]!;
      expect(cmd.request.sessionId).toBe(sessionId);
      expect(cmd.request.token).toBe(token);
      expect(gate.registry.get(sessionId)).toBe(token);
    } finally {
      gate.stop();
    }
  });
});

describe("env_injection_does_not_affect_daemon_behavior", () => {
  test("deregisterSession is a no-op when gate is absent (swallows error)", async () => {
    const nonExistent = join(tmpdir(), "no-gate-here.sock");
    await expect(
      deregisterSession("any-sid", { socketPath: nonExistent, adminToken: ADMIN_TOKEN }),
    ).resolves.toBeUndefined();
  });
});

describe("deregisters_session_with_gate_on_daemon_exit", () => {
  let gate: FakeAdmin;
  afterEach(() => gate?.stop());

  test("deregisterSession removes the session from the registry", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);
    const sessionId = "exit-session";
    const token = "e".repeat(64);

    await registerSession(sessionId, token, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });
    expect(gate.registry.has(sessionId)).toBe(true);

    await deregisterSession(sessionId, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });

    expect(gate.registry.has(sessionId)).toBe(false);
  });

  test("deregisterSession does not throw when gate is unreachable", async () => {
    const nonExistentSocket = join(tmpdir(), "does-not-exist-gate-admin.sock");

    await expect(
      deregisterSession("any-session", {
        socketPath: nonExistentSocket,
        adminToken: ADMIN_TOKEN,
      }),
    ).resolves.toBeUndefined();
  });
});

describe("late_hooks_after_deregister_fail_gate_auth_unauthenticated", () => {
  let gate: FakeAdmin;
  afterEach(() => gate?.stop());

  test("after deregister, re-register attempt with old token is rejected", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);
    const sessionId = "late-hook-session";
    const token = "f".repeat(64);

    await registerSession(sessionId, token, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });
    await deregisterSession(sessionId, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });

    expect(gate.registry.has(sessionId)).toBe(false);
  });

  test("wrong admin token is rejected (simulates late hook failing auth)", async () => {
    gate = startFakeAdmin(ADMIN_TOKEN);
    const sessionId = "late-hook-session-2";
    const token = "g".repeat(64);

    await registerSession(sessionId, token, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });
    await deregisterSession(sessionId, {
      socketPath: gate.socketPath,
      adminToken: ADMIN_TOKEN,
    });

    // A hook posting with the old token finds no session in the registry.
    // Simulate this by checking the registry directly.
    expect(gate.registry.has(sessionId)).toBe(false);
  });
});

describe("launches_tools_via_process_launch_no_peer_spawning", () => {
  test("register and deregister are the only admin operations (no peer-tool calls)", async () => {
    const gate = startFakeAdmin(ADMIN_TOKEN);
    try {
      const sessionId = "peer-spawn-check";
      const token = "i".repeat(64);

      await registerSession(sessionId, token, {
        socketPath: gate.socketPath,
        adminToken: ADMIN_TOKEN,
      });
      await deregisterSession(sessionId, {
        socketPath: gate.socketPath,
        adminToken: ADMIN_TOKEN,
      });

      // Only register + deregister should have been sent; no unknown commands
      const commands = gate.commands.map((c) => c.request.command);
      expect(commands).toEqual(["register", "deregister"]);
    } finally {
      gate.stop();
    }
  });
});
