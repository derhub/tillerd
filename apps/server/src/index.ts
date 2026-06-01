import { createEngine } from "@athing/engine";
import { claudeCode } from "@athing/adapter-claude-code";
import * as v from "valibot";
import type { AgentSession } from "@athing/sdk";

const ClientMessageSchema = v.union([
  v.object({ type: v.literal("send"), text: v.string() }),
  v.object({ type: v.literal("input"), bytes: v.array(v.number()) }),
  v.object({ type: v.literal("interrupt") }),
  v.object({ type: v.literal("resize"), cols: v.number(), rows: v.number() }),
  v.object({ type: v.literal("kill") }),
]);

type ClientMessage = v.InferOutput<typeof ClientMessageSchema>;

interface WsData {
  sessionId: string;
}

const engine = createEngine();
const activeSessions = new Map<string, AgentSession>();

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
      const upgraded = server.upgrade(req, { data: { sessionId: "" } });
      if (!upgraded) return new Response("upgrade failed", { status: 400 });
      return undefined;
    }

    return Response.json({ error: "Not Found" }, { status: 404, headers: corsHeaders });
  },
  websocket: {
    async open(ws) {
      const session = await engine.start(claudeCode, {
        cwd: process.cwd(),
        cols: 220,
        rows: 50,
      });

      ws.data.sessionId = session.sessionId;
      activeSessions.set(session.sessionId, session);

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
        ws.send(JSON.stringify({ type: "exit", ...event }));
        ws.close();
      });
    },

    message(ws, raw) {
      const session = activeSessions.get(ws.data.sessionId);
      if (!session) return;

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
      const session = activeSessions.get(ws.data.sessionId);
      if (session) {
        activeSessions.delete(ws.data.sessionId);
        void session.kill();
      }
    },
  },
});

console.log(`Server on http://localhost:${process.env["PORT"] ?? 3000}`);
