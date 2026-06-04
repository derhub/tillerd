import { createEngine } from "@athing/engine";
import { claudeCode } from "@athing/adapter-claude-code";
import { Database } from "bun:sqlite";
import * as v from "valibot";
import * as fs from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { randomUUID, randomBytes } from "node:crypto";
import { createLogger } from "@athing/logger";
import type { AgentSession, DaemonTransport } from "@athing/sdk";
import { ATTR } from "@athing/sdk";
import {
  adoptOrSpawn,
  agentHome,
  BunFileSource,
  checkCliVersion,
  HOOKS_SOCK,
  resolveAgentCommand,
} from "@athing/platform-bun";

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
  v.object({ type: v.literal("stop") }),
  v.object({ type: v.literal("spawn"), resume: v.string() }),
]);

type ClientMessage = v.InferOutput<typeof ClientMessageSchema>;

// ── Startup bootstrap: resolve host concerns, then inject into the engine ─────

const logger = createLogger({
  "service.name": "athing-server",
  "service.version": "0.0.1",
  "process.pid": process.pid,
});
const log = logger.child({ [ATTR.COMPONENT]: "server" });

// Verify the agent CLI version before serving. Adapter setup (hook install) is the
// installer's responsibility; the server assumes it is already complete.
checkCliVersion(claudeCode.launch.command, claudeCode.cliVersionRange);
const resolvedCommand = resolveAgentCommand(claudeCode.binaryResolution);

// One daemon connection, shared by the engine (agent sessions) and the raw
// terminal path. The host owns its lifecycle.
const transport: DaemonTransport = await adoptOrSpawn();
let ownedDaemonPid: number | null = null;
try {
  const raw = JSON.parse(fs.readFileSync(join(ATHING_DIR, "daemon.json"), "utf8")) as {
    pid: number;
  };
  ownedDaemonPid = raw.pid;
} catch {
  /* manifest not readable */
}
transport.onClose(() => {
  log.warn("daemon disconnected");
  ownedDaemonPid = null;
});

const fileSource = new BunFileSource();
const engine = createEngine({
  transport,
  fileSource,
  logger,
  hooksSocketPath: HOOKS_SOCK,
  agentHome: agentHome(),
  resolvedCommand,
});

function shutdownDaemon(): void {
  const pid = ownedDaemonPid;
  ownedDaemonPid = null;
  try {
    transport.disconnect();
  } catch {
    /* already closed */
  }
  if (!pid) return;
  try {
    process.kill(pid, "SIGTERM");
    log.info("sent SIGTERM to daemon", { pid });
  } catch {
    /* already dead */
  }
}

for (const sig of ["SIGINT", "SIGTERM"] as const) {
  process.on(sig, () => {
    log.info("shutting down daemon on signal", { signal: sig });
    shutdownDaemon();
    process.exit(0);
  });
}

process.on("exit", () => {
  shutdownDaemon();
});

process.on("uncaughtException", (err) => {
  log.error("uncaught exception — shutting down daemon", { err: String(err) });
  shutdownDaemon();
  process.exit(1);
});

interface WsData {
  mode: "session" | "terminal";
  sessionId: string;
  reconnectId: string | null;
  _termUnsub: (() => void) | null;
}

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
      log.info("daemon has reconnectable sessions", { count: liveIds.length });
    }
  } catch {
    // daemon not yet running — that's fine
  }
})();

Bun.serve<WsData>({
  port: Number(process.env["PORT"] ?? 3000),
  async fetch(req, server) {
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

    const diffMatch = url.pathname.match(/^\/api\/sessions\/([^/]+)\/diff$/);
    if (diffMatch && req.method === "GET") {
      const sessionId = diffMatch[1] as string;
      const row = db.query("SELECT cwd FROM sessions WHERE id = ?").get(sessionId) as {
        cwd: string;
      } | null;
      if (!row) {
        return Response.json({ error: "session not found" }, { status: 404, headers: corsHeaders });
      }
      try {
        const proc = Bun.spawn(["git", "diff", "HEAD"], {
          cwd: row.cwd,
          stdout: "pipe",
          stderr: "ignore",
        });
        const output = await new Response(proc.stdout).text();
        await proc.exited;
        return new Response(output, {
          headers: { ...corsHeaders, "Content-Type": "text/plain; charset=utf-8" },
        });
      } catch {
        return new Response("", {
          headers: { ...corsHeaders, "Content-Type": "text/plain; charset=utf-8" },
        });
      }
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

        const token = randomBytes(32).toString("hex");
        const tlog = logger.child({ [ATTR.COMPONENT]: "server", [ATTR.SESSION_ID]: sessionId });

        let dataFrameCount = 0;
        const unsub = transport.subscribe(sessionId, (frame, body) => {
          if (frame.type === "data") {
            const bytes = body ? Array.from(new Uint8Array(body)) : [];
            dataFrameCount++;
            tlog.debug("terminal data frame", {
              [ATTR.FRAME_SEQ]: dataFrameCount,
              bytes: bytes.length,
            });
            ws.send(JSON.stringify({ type: "data", bytes }));
            // Return flow-control credit so the daemon doesn't stall.
            transport.send({ type: "ack", sessionId, bytes: bytes.length });
          } else if (frame.type === "exit") {
            tlog.info("terminal exit", { qualifier: frame.qualifier });
            ws.send(JSON.stringify({ type: "exit", qualifier: frame.qualifier, raw: frame.raw }));
            ws.close();
          } else {
            tlog.debug("terminal frame", { frameType: frame.type });
          }
        });
        ws.data._termUnsub = unsub;

        tlog.info("terminal spawn", { cwd });
        transport.send({
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
        transport.send({ type: "subscribe", sessionId });

        ws.send(JSON.stringify({ type: "session_start", sessionId }));
        tlog.info("terminal subscribed");
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
        const sid = ws.data.sessionId;
        switch (msg.type) {
          case "input":
            transport.send({ type: "input", sessionId: sid }, Buffer.from(msg.bytes));
            break;
          case "resize":
            transport.send({ type: "resize", sessionId: sid, cols: msg.cols, rows: msg.rows });
            break;
          case "interrupt":
            transport.send({ type: "interrupt", sessionId: sid });
            break;
          case "kill":
            transport.send({ type: "kill", sessionId: sid });
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
        case "stop":
          void session.stop();
          break;
        case "spawn": {
          // Recovery: re-bind the WebSocket to a new session resumed from the crashed one.
          const resumeId = msg.resume;
          activeSessions.delete(ws.data.sessionId);
          void (async () => {
            try {
              const recovered = await engine.start(claudeCode, {
                cwd: process.cwd(),
                cols: 220,
                rows: 50,
                resume: resumeId,
              });
              ws.data.sessionId = recovered.sessionId;
              activeSessions.set(recovered.sessionId, recovered);
              db.run("INSERT OR IGNORE INTO sessions (id, cwd, created_at) VALUES (?, ?, ?)", [
                recovered.sessionId,
                process.cwd(),
                Date.now(),
              ]);
              ws.send(JSON.stringify({ type: "session_start", sessionId: recovered.sessionId }));
              recovered.onData((bytes) =>
                ws.send(JSON.stringify({ type: "data", bytes: Array.from(bytes) })),
              );
              recovered.onStatus((status) => ws.send(JSON.stringify({ type: "status", status })));
              recovered.onContent((event) => ws.send(JSON.stringify({ type: "content", event })));
              recovered.onError((err) =>
                ws.send(JSON.stringify({ type: "error", kind: err.kind, message: err.message })),
              );
              recovered.onExit((event) => {
                activeSessions.delete(recovered.sessionId);
                db.run("DELETE FROM sessions WHERE id = ?", [recovered.sessionId]);
                ws.send(JSON.stringify({ type: "exit", ...event }));
                ws.close();
              });
            } catch (err) {
              const e = err instanceof Error ? err : new Error(String(err));
              ws.send(
                JSON.stringify({
                  type: "error",
                  kind: (err as { kind?: string }).kind ?? "SpawnFailed",
                  message: e.message,
                }),
              );
            }
          })();
          break;
        }
      }
    },

    close(ws) {
      if (ws.data.mode === "terminal") {
        ws.data._termUnsub?.();
        if (ws.data.sessionId) {
          log.info("terminal close → kill", { [ATTR.SESSION_ID]: ws.data.sessionId });
          transport.send({ type: "kill", sessionId: ws.data.sessionId });
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
