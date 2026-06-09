import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { HOOK_SUBSCRIPTION_WIRE_VERSION } from "@athing/sdk";

/** Resolve the gate's single socket path from ATHING_DIR; the admin face is reached
 * over its `Admin` route. */
function gateAdminSock(): string {
  const raw = process.env["ATHING_DIR"];
  const dir = raw ? resolve(raw) : join(homedir(), ".athing");
  return join(dir, "gate.sock");
}

/**
 * Write a 4-byte big-endian length-prefixed frame to a buffer.
 * Matches the gate's loopback IPC framing (D9).
 */
function encodeFrame(payload: Uint8Array): Uint8Array {
  const out = new Uint8Array(4 + payload.byteLength);
  const view = new DataView(out.buffer);
  view.setUint32(0, payload.byteLength, false);
  out.set(payload, 4);
  return out;
}

interface AdminResponse {
  result: string;
  reason?: string;
}

async function sendAdminCommand(
  socketPath: string,
  adminToken: string,
  command: object,
): Promise<AdminResponse> {
  return new Promise((resolve, reject) => {
    // Open the gate socket on the Admin route: one preamble frame carrying the admin
    // token, then one bare command frame.
    const preambleFrame = encodeFrame(
      new TextEncoder().encode(
        JSON.stringify({
          route: "admin",
          token: adminToken,
          wireVersion: HOOK_SUBSCRIPTION_WIRE_VERSION,
        }),
      ),
    );
    const commandFrame = encodeFrame(new TextEncoder().encode(JSON.stringify(command)));

    let headerBuf: Uint8Array | null = null;
    let payloadBuf: Uint8Array | null = null;
    let payloadFilled = 0;

    function processChunk(chunk: Uint8Array): AdminResponse | null {
      if (!headerBuf) {
        if (chunk.length < 4) return null;
        const view = new DataView(chunk.buffer, chunk.byteOffset, chunk.byteLength);
        const len = view.getUint32(0, false);
        headerBuf = chunk.slice(0, 4);
        payloadBuf = new Uint8Array(len);
        payloadFilled = 0;
        const rest = chunk.slice(4);
        if (rest.length > 0) {
          const toCopy = Math.min(rest.length, len);
          payloadBuf.set(rest.slice(0, toCopy), 0);
          payloadFilled = toCopy;
        }
      } else if (payloadBuf) {
        const needed = payloadBuf.byteLength - payloadFilled;
        const toCopy = Math.min(chunk.length, needed);
        payloadBuf.set(chunk.slice(0, toCopy), payloadFilled);
        payloadFilled += toCopy;
      }

      if (payloadBuf && payloadFilled >= payloadBuf.byteLength) {
        const text = new TextDecoder().decode(payloadBuf);
        return JSON.parse(text) as AdminResponse;
      }
      return null;
    }

    Bun.connect({
      unix: socketPath,
      socket: {
        open(socket) {
          socket.write(preambleFrame);
          socket.write(commandFrame);
        },
        data(_socket, data: Buffer) {
          const chunk = new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
          const resp = processChunk(chunk);
          if (resp) resolve(resp);
        },
        close() {
          reject(new Error("gate-admin: connection closed before response"));
        },
        error(_socket, err) {
          reject(err);
        },
        connectError(_socket, err) {
          reject(err);
        },
      },
    });
  });
}

export interface GateAdminOptions {
  /** Path to the gate socket. Defaults to $ATHING_DIR/gate.sock. */
  socketPath?: string;
  /** Admin token. Reads ATHING_GATE_ADMIN_TOKEN from env when absent. */
  adminToken?: string;
}

function resolveAdminToken(opts: GateAdminOptions): string {
  const tok = opts.adminToken ?? process.env["ATHING_GATE_ADMIN_TOKEN"];
  if (!tok) throw new Error("gate-admin: ATHING_GATE_ADMIN_TOKEN is not set");
  return tok;
}

/** Register a session with the gate admin before spawning the daemon (R4/D7). */
export async function registerSession(
  sessionId: string,
  token: string,
  opts: GateAdminOptions = {},
): Promise<void> {
  const socketPath = opts.socketPath ?? gateAdminSock();
  const adminToken = resolveAdminToken(opts);
  const resp = await sendAdminCommand(socketPath, adminToken, {
    command: "register",
    sessionId,
    token,
  });
  if (resp.result !== "ok") {
    throw new Error(`gate-admin register failed: ${resp.result} ${resp.reason ?? ""}`);
  }
}

/** Deregister a session after the daemon PTY session exits. */
export async function deregisterSession(
  sessionId: string,
  opts: GateAdminOptions = {},
): Promise<void> {
  const socketPath = opts.socketPath ?? gateAdminSock();
  const adminToken = resolveAdminToken(opts);
  try {
    const resp = await sendAdminCommand(socketPath, adminToken, {
      command: "deregister",
      sessionId,
    });
    if (resp.result !== "ok") {
      // Log but do not throw — deregister is best-effort on exit
      console.warn(`gate-admin deregister: ${resp.result} ${resp.reason ?? ""}`);
    }
  } catch {
    // Unreachable gate on exit: swallow (late hooks will fail Auth anyway)
  }
}
