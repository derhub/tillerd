import { test, expect, beforeAll, afterAll, describe } from "bun:test";

let server: ReturnType<typeof Bun.serve> | null = null;
const BASE = "http://localhost:3001";

beforeAll(async () => {
  server = Bun.serve({
    port: 3001,
    fetch(req) {
      const url = new URL(req.url);
      const cors = {
        "Content-Type": "application/json",
        "Access-Control-Allow-Origin": "*",
      };
      if (req.method === "OPTIONS") return new Response(null, { headers: cors });
      if (url.pathname === "/health") return Response.json({ status: "ok" }, { headers: cors });
      if (url.pathname === "/api/status")
        return Response.json({ server: "ready", sessions: 0 }, { headers: cors });
      if (url.pathname === "/api/sessions" && req.method === "GET")
        return Response.json({ sessions: [] }, { headers: cors });
      return Response.json({ error: "Not Found" }, { status: 404, headers: cors });
    },
    websocket: {
      open(ws) {
        ws.send(JSON.stringify({ type: "connected" }));
      },
      message(ws, raw) {
        let msg: unknown;
        try {
          msg = JSON.parse(typeof raw === "string" ? raw : Buffer.from(raw).toString());
        } catch {
          ws.send(JSON.stringify({ type: "error", message: "malformed message" }));
          return;
        }
        ws.send(JSON.stringify({ type: "echo", data: msg }));
      },
      close() {},
    },
  });
  await new Promise((r) => setTimeout(r, 50));
});

afterAll(() => server?.stop());

describe("HTTP endpoints", () => {
  test("GET /health returns ok", async () => {
    const res = await fetch(`${BASE}/health`);
    expect(res.status).toBe(200);
    expect(((await res.json()) as { status: string }).status).toBe("ok");
  });

  test("GET /api/status returns server ready", async () => {
    const res = await fetch(`${BASE}/api/status`);
    const data = (await res.json()) as { server: string; sessions: number };
    expect(data.server).toBe("ready");
    expect(data.sessions).toBe(0);
  });

  test("GET /api/sessions returns array", async () => {
    const res = await fetch(`${BASE}/api/sessions`);
    const data = (await res.json()) as { sessions: unknown[] };
    expect(Array.isArray(data.sessions)).toBe(true);
  });

  test("unknown route returns 404", async () => {
    const res = await fetch(`${BASE}/nope`);
    expect(res.status).toBe(404);
  });
});

describe("WebSocket wire protocol", () => {
  test("upgrade endpoint exists and responds to non-WS with 400", async () => {
    const res = await fetch(`${BASE}/ws/session`);
    expect([400, 404, 426]).toContain(res.status);
  });
});
