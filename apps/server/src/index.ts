import { createEngine, adoptOrSpawn, DaemonClient } from "@athing/engine";
import { claudeCode } from "@athing/adapter-claude-code";
import { HOOKS_SOCK } from "@athing/daemon";
import { Database } from "bun:sqlite";
import * as v from "valibot";
import * as fs from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { randomUUID, randomBytes } from "node:crypto";
import type { AgentSession } from "@athing/sdk";

const ATHING_DIR = join(homedir(), ".athing");
fs.mkdirSync(ATHING_DIR, { recursive: true });

const db = new Database(join(ATHING_DIR, "server.db"));
db.run(`CREATE TABLE IF NOT EXISTS sessions (
  id TEXT PRIMARY KEY,
  cwd TEXT NOT NULL,
  created_at INTEGER NOT NULL
)`);

const ClientMessageSchema = v.union([
  v.object({ type: v.literal("send"), text: v.string() }),
  v.object({ type: v.literal("input"), bytes: v.array(v.number()) }),
  v.object({ type: v.literal("interrupt") }),
  v.object({ type: v.literal("resize"), cols: v.number(), rows: v.number() }),
  v.object({ type: v.literal("kill") }),
]);

type ClientMessage = v.InferOutput<typeof ClientMessageSchema>;

// Shared daemon client for raw terminal sessions
let terminalClient: DaemonClient | null = null;
let terminalClientPromise: Promise<DaemonClient> | null = null;
let ownedDaemonPid: number | null = null; // non-null if this server spawned the daemon

function getTerminalClient(): Promise<DaemonClient> {
  if (!terminalClientPromise) {
    terminalClientPromise = adoptOrSpawn().then((c) => {
      terminalClient = c;
      // Track whether we spawned a new daemon (manifest written after our connect).
      try {
        const raw = JSON.parse(fs.readFileSync(join(ATHING_DIR, "daemon.json"), "utf8")) as {
          pid: number;
        };
        ownedDaemonPid = raw.pid;
      } catch {
        /* manifest not readable */
      }
      c.onClose(() => {
        console.log("[terminal] daemon disconnected — clearing client cache");
        terminalClient = null;
        terminalClientPromise = null;
        ownedDaemonPid = null;
      });
      return c;
    });
  }
  return terminalClientPromise;
}

function shutdownDaemon(): void {
  const pid = ownedDaemonPid;
  if (!pid) return;
  terminalClient?.disconnect();
  terminalClient = null;
  terminalClientPromise = null;
  ownedDaemonPid = null;
  try {
    process.kill(pid, "SIGTERM");
    console.log("[server] sent SIGTERM to daemon", pid);
  } catch {
    /* already dead */
  }
}

for (const sig of ["SIGINT", "SIGTERM"] as const) {
  process.on(sig, () => {
    console.log(`[server] ${sig} — shutting down daemon`);
    shutdownDaemon();
    process.exit(0);
  });
}

interface WsData {
  mode: "session" | "terminal";
  sessionId: string;
  reconnectId: string | null;
  _termUnsub: (() => void) | null;
}

const engine = createEngine();
const activeSessions = new Map<string, AgentSession>();

// On startup: reconcile DB against live daemon sessions
(async () => {
  try {
    const liveIds = await engine.listSessions();
    const liveSet = new Set(liveIds);
    const dbRows = db.query("SELECT id FROM sessions").all() as Array<{ id: string }>;
    for (const row of dbRows) {
      if (!liveSet.has(row.id)) {
        db.run("DELETE FROM sessions WHERE id = ?", [row.id]);
      }
    }
    if (liveIds.length > 0) {
      console.log(`Daemon has ${liveIds.length} reconnectable session(s)`);
    }
  } catch {
    // daemon not yet running — that's fine
  }
})();

Bun.serve<WsData>({
  port: Number(process.env["PORT"] ?? 3000),
  fetch(req, server) {
    const url = new URL(req.url);

    const corsHeaders = {
      "Access-Control-Allow-Origin": "*",
      "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      "Access-Control-Allow-Headers": "Content-Type",
    };

    if (req.method === "OPTIONS") {
      return new Response(null, { headers: corsHeaders });
    }

    if (url.pathname === "/health") {
      return Response.json({ status: "ok" }, { headers: corsHeaders });
    }

    if (url.pathname === "/api/status") {
      return Response.json(
        { server: "ready", sessions: activeSessions.size },
        { headers: corsHeaders },
      );
    }

    if (url.pathname === "/api/sessions" && req.method === "GET") {
      return Response.json(
        { sessions: [...activeSessions.keys()].map((id) => ({ id })) },
        { headers: corsHeaders },
      );
    }

    if (url.pathname === "/ws/session") {
      const reconnectId = url.searchParams.get("id");
      const upgraded = server.upgrade(req, {
        data: { mode: "session", sessionId: "", reconnectId, _termUnsub: null },
      });
      if (!upgraded) return new Response("upgrade failed", { status: 400 });
      return undefined;
    }

    if (url.pathname === "/ws/terminal") {
      const upgraded = server.upgrade(req, {
        data: { mode: "terminal", sessionId: "", reconnectId: null, _termUnsub: null },
      });
      if (!upgraded) return new Response("upgrade failed", { status: 400 });
      return undefined;
    }

    return Response.json({ error: "Not Found" }, { status: 404, headers: corsHeaders });
  },
  websocket: {
    async open(ws) {
      const cwd = process.cwd();

      if (ws.data.mode === "terminal") {
        const sessionId = randomUUID();
        ws.data.sessionId = sessionId;

        const client = await getTerminalClient();
        const token = randomBytes(32).toString("hex");

        let dataFrameCount = 0;
        const unsub = client.subscribe(sessionId, (frame, body) => {
          if (frame.type === "data") {
            const bytes = body ? Array.from(new Uint8Array(body)) : [];
            dataFrameCount++;
            console.log(
              "[terminal] data frame",
              sessionId.slice(0, 8),
              `#${dataFrameCount}`,
              `${bytes.length}b`,
            );
            ws.send(JSON.stringify({ type: "data", bytes }));
            // Return flow-control credit so the daemon doesn't stall.
            client.send({ type: "ack", sessionId, bytes: bytes.length });
          } else if (frame.type === "exit") {
            console.log("[terminal] exit", sessionId.slice(0, 8), frame.code, frame.signal);
            ws.send(JSON.stringify({ type: "exit", code: frame.code, signal: frame.signal }));
            ws.close();
          } else {
            console.log("[terminal] frame", sessionId.slice(0, 8), frame.type);
          }
        });
        ws.data._termUnsub = unsub;

        console.log("[terminal] spawn", sessionId.slice(0, 8), { cwd });
        client.send({
          type: "spawn",
          sessionId,
          command: process.env["SHELL"] ?? "/bin/zsh",
          args: [], // empty → pty-transport uses direct interactive spawn
          flags: [],
          hookSocketPath: HOOKS_SOCK,
          token,
          cols: 220,
          rows: 50,
          cwd,
        });

        // Tell the daemon to start streaming output for this session.
        client.send({ type: "subscribe", sessionId });

        ws.send(JSON.stringify({ type: "session_start", sessionId }));
        console.log("[terminal] subscribed", sessionId.slice(0, 8));
        return;
      }

      if (ws.data.reconnectId) {
        const sessionId = ws.data.reconnectId;
        const row = db.query("SELECT cwd FROM sessions WHERE id = ?").get(sessionId) as {
          cwd: string;
        } | null;
        const sessionCwd = row?.cwd ?? cwd;

        try {
          const session = await engine.reconnect(sessionId, claudeCode, {
            cwd: sessionCwd,
            cols: 220,
            rows: 50,
          });

          ws.data.sessionId = session.sessionId;
          activeSessions.set(session.sessionId, session);

          ws.send(JSON.stringify({ type: "session_resume", sessionId: session.sessionId }));

          session.onData((bytes) => {
            ws.send(JSON.stringify({ type: "data", bytes: Array.from(bytes) }));
          });

          session.onStatus((status) => {
            ws.send(JSON.stringify({ type: "status", status }));
          });

          session.onContent((event) => {
            ws.send(JSON.stringify({ type: "content", event }));
          });

          session.onError((err) => {
            ws.send(JSON.stringify({ type: "error", kind: err.kind, message: err.message }));
          });

          session.onExit((event) => {
            activeSessions.delete(session.sessionId);
            db.run("DELETE FROM sessions WHERE id = ?", [session.sessionId]);
            ws.send(JSON.stringify({ type: "exit", ...event }));
            ws.close();
          });
        } catch {
          ws.send(
            JSON.stringify({
              type: "error",
              kind: "TransportClosed",
              message: "Session not found or dead",
            }),
          );
          ws.close();
        }
        return;
      }

      const session = await engine.start(claudeCode, { cwd, cols: 220, rows: 50 });

      ws.data.sessionId = session.sessionId;
      activeSessions.set(session.sessionId, session);
      db.run("INSERT OR IGNORE INTO sessions (id, cwd, created_at) VALUES (?, ?, ?)", [
        session.sessionId,
        cwd,
        Date.now(),
      ]);

      ws.send(JSON.stringify({ type: "session_start", sessionId: session.sessionId }));

      session.onData((bytes) => {
        ws.send(JSON.stringify({ type: "data", bytes: Array.from(bytes) }));
      });

      session.onStatus((status) => {
        ws.send(JSON.stringify({ type: "status", status }));
      });

      session.onContent((event) => {
        ws.send(JSON.stringify({ type: "content", event }));
      });

      session.onError((err) => {
        ws.send(JSON.stringify({ type: "error", kind: err.kind, message: err.message }));
      });

      session.onExit((event) => {
        activeSessions.delete(session.sessionId);
        db.run("DELETE FROM sessions WHERE id = ?", [session.sessionId]);
        ws.send(JSON.stringify({ type: "exit", ...event }));
        ws.close();
      });
    },

    message(ws, raw) {
      let msg: ClientMessage;
      try {
        const parsed = JSON.parse(
          typeof raw === "string" ? raw : Buffer.from(raw).toString("utf8"),
        );
        msg = v.parse(ClientMessageSchema, parsed);
      } catch {
        ws.send(JSON.stringify({ type: "error", kind: "parse", message: "malformed message" }));
        return;
      }

      if (ws.data.mode === "terminal") {
        if (!terminalClient) return;
        const sid = ws.data.sessionId;
        switch (msg.type) {
          case "input":
            terminalClient.send({ type: "input", sessionId: sid }, Buffer.from(msg.bytes));
            break;
          case "resize":
            terminalClient.send({ type: "resize", sessionId: sid, cols: msg.cols, rows: msg.rows });
            break;
          case "interrupt":
            terminalClient.send({ type: "interrupt", sessionId: sid });
            break;
          case "kill":
            terminalClient.send({ type: "kill", sessionId: sid });
            break;
        }
        return;
      }

      const session = activeSessions.get(ws.data.sessionId);
      if (!session) return;

      switch (msg.type) {
        case "send":
          session.send(msg.text);
          break;
        case "input":
          session.input(new Uint8Array(msg.bytes));
          break;
        case "interrupt":
          session.interrupt();
          break;
        case "resize":
          session.resize(msg.cols, msg.rows);
          break;
        case "kill":
          void session.kill();
          break;
      }
    },

    close(ws) {
      if (ws.data.mode === "terminal") {
        ws.data._termUnsub?.();
        if (terminalClient && ws.data.sessionId) {
          console.log("[terminal] close → kill", ws.data.sessionId.slice(0, 8));
          terminalClient.send({ type: "kill", sessionId: ws.data.sessionId });
        }
        return;
      }
      // Remove from active map but do NOT kill — session persists in the daemon
      // so the client can reconnect with ?id= and resume.
      activeSessions.delete(ws.data.sessionId);
    },
  },
});

console.log(`Server on http://localhost:${process.env["PORT"] ?? 3000}`);
