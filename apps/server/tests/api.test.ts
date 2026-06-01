import { test, expect, beforeAll, afterAll } from "bun:test";

let server: ReturnType<typeof Bun.serve> | null = null;
const baseURL = "http://localhost:3001";

beforeAll(async () => {
  // Start test server on different port
  server = Bun.serve({
    port: 3001,
    fetch(req) {
      const url = new URL(req.url);

      const headers = {
        "Content-Type": "application/json",
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
        "Access-Control-Allow-Headers": "Content-Type",
      };

      if (req.method === "OPTIONS") {
        return new Response(null, { headers });
      }

      if (url.pathname === "/health") {
        return new Response(JSON.stringify({ status: "ok", timestamp: Date.now() }), {
          headers,
        });
      }

      if (url.pathname === "/api/status") {
        return new Response(JSON.stringify({ server: "ready", sessions: 0 }), {
          headers,
        });
      }

      if (url.pathname === "/api/sessions") {
        if (req.method === "GET") {
          return new Response(JSON.stringify({ sessions: [] }), { headers });
        }
        if (req.method === "POST") {
          const id = `session-${Date.now()}`;
          return new Response(JSON.stringify({ id, status: "running" }), {
            status: 201,
            headers,
          });
        }
      }

      return new Response(JSON.stringify({ error: "Not Found" }), {
        status: 404,
        headers,
      });
    },
  });

  // Wait for server to be ready
  await new Promise((resolve) => setTimeout(resolve, 100));
});

afterAll(() => {
  if (server) {
    server.stop();
  }
});

test("GET /health returns ok", async () => {
  const response = await fetch(`${baseURL}/health`);
  expect(response.status).toBe(200);

  const data = await response.json();
  expect(data.status).toBe("ok");
  expect(data.timestamp).toBeDefined();
});

test("GET /api/status returns server status", async () => {
  const response = await fetch(`${baseURL}/api/status`);
  expect(response.status).toBe(200);

  const data = await response.json();
  expect(data.server).toBe("ready");
  expect(data.sessions).toBe(0);
});

test("GET /api/sessions returns sessions array", async () => {
  const response = await fetch(`${baseURL}/api/sessions`);
  expect(response.status).toBe(200);

  const data = await response.json();
  expect(Array.isArray(data.sessions)).toBe(true);
});

test("POST /api/sessions creates session", async () => {
  const response = await fetch(`${baseURL}/api/sessions`, {
    method: "POST",
  });
  expect(response.status).toBe(201);

  const data = await response.json();
  expect(data.id).toBeDefined();
  expect(data.status).toBe("running");
});

test("404 for unknown routes", async () => {
  const response = await fetch(`${baseURL}/unknown`);
  expect(response.status).toBe(404);

  const data = await response.json();
  expect(data.error).toBe("Not Found");
});
