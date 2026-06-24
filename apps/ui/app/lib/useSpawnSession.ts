import { useQueryClient } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import React from "react";

import { scalarString } from "~/lib/json";
import { WS_BASE } from "~/lib/serverUrl";

export function useSpawnSession() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [spawning, setSpawning] = React.useState(false);
  const wsRef = React.useRef<WebSocket | null>(null);

  const spawn = React.useCallback(() => {
    if (spawning) return;
    setSpawning(true);

    const ws = new WebSocket(`${WS_BASE}/ws/session`);
    wsRef.current = ws;

    ws.onmessage = (e) => {
      const msg = JSON.parse(e.data as string) as Record<string, unknown>;
      if (msg["type"] === "session_start") {
        const id = scalarString(msg["sessionId"]);
        ws.close();
        wsRef.current = null;
        setSpawning(false);
        void queryClient.invalidateQueries({ queryKey: ["sessions"] });
        void navigate({ to: `/session/${id}` } as never);
      }
    };

    ws.onerror = () => {
      setSpawning(false);
      wsRef.current = null;
    };

    ws.onclose = () => {
      setSpawning(false);
    };
  }, [spawning, navigate, queryClient]);

  return { spawn, spawning };
}
