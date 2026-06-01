const sessions: Array<{ id: string; status: string }> = [];

Bun.serve({
  port: 3000,
  fetch(req) {
    const url = new URL(req.url);

    // CORS headers
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
      return new Response(JSON.stringify({ server: "ready", sessions: sessions.length }), {
        headers,
      });
    }

    if (url.pathname === "/api/sessions") {
      if (req.method === "GET") {
        return new Response(JSON.stringify({ sessions }), { headers });
      }
      if (req.method === "POST") {
        const id = `session-${Date.now()}`;
        sessions.push({ id, status: "running" });
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
  websocket: {
    open(ws) {
      ws.send(JSON.stringify({ type: "connected" }));
    },
    message(ws, message) {
      ws.send(JSON.stringify({ type: "echo", data: message }));
    },
    close() {},
  },
});

console.log("Server running on http://localhost:3000");
