import { useState, useCallback, useRef } from "react";
import { useNavigate, useRevalidator } from "react-router";

const WS_BASE = `ws://${typeof window !== "undefined" ? window.location.hostname : "localhost"}:3000`;

export function useSpawnSession() {
  const navigate = useNavigate();
  const revalidator = useRevalidator();
  const [spawning, setSpawning] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);

  const spawn = useCallback(() => {
    if (spawning) return;
    setSpawning(true);

    const ws = new WebSocket(`${WS_BASE}/ws/session`);
    wsRef.current = ws;

    ws.onmessage = (e) => {
      const msg = JSON.parse(e.data as string) as Record<string, unknown>;
      if (msg["type"] === "session_start") {
        const id = String(msg["sessionId"] ?? "");
        ws.close();
        wsRef.current = null;
        setSpawning(false);
        revalidator.revalidate();
        void navigate(`/session/${id}`);
      }
    };

    ws.onerror = () => {
      setSpawning(false);
      wsRef.current = null;
    };

    ws.onclose = () => {
      if (spawning) setSpawning(false);
    };
  }, [spawning, navigate, revalidator]);

  return { spawn, spawning };
}
