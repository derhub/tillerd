import { useEffect, useRef, useState, useCallback } from "react";
import "@xterm/xterm/css/xterm.css";
import type { Route } from "./+types/_index";

export const meta: Route.MetaFunction = () => [
  { title: "Terminal | a-thing" },
  { name: "description", content: "Interactive PTY terminal" },
];

const WS_URL = `ws://${typeof window !== "undefined" ? window.location.hostname : "localhost"}:3000/ws/terminal`;

type Status = "connecting" | "connected" | "disconnected";

export default function TerminalPage() {
  const termContainerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<import("@xterm/xterm").Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [status, setStatus] = useState<Status>("connecting");

  const openWs = useCallback(() => {
    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;
    setStatus("connecting");
    setSessionId(null);

    ws.onopen = () => setStatus("connected");
    ws.onclose = () => {
      setStatus("disconnected");
      setSessionId(null);
      termRef.current?.write("\r\n\x1b[31m[disconnected]\x1b[0m\r\n");
    };
    ws.onerror = () => {
      termRef.current?.write(
        "\r\n\x1b[31m[connection error — is the server running on :3000?]\x1b[0m\r\n",
      );
    };
    ws.onmessage = (e) => {
      const msg = JSON.parse(e.data as string) as Record<string, unknown>;
      switch (msg["type"]) {
        case "session_start":
          setSessionId(String(msg["sessionId"] ?? ""));
          break;
        case "data": {
          const arr = new Uint8Array(msg["bytes"] as number[]);
          termRef.current?.write(arr);
          break;
        }
        case "exit":
          termRef.current?.write(
            `\r\n\x1b[33m[exited code=${msg["code"] ?? "?"} signal=${msg["signal"] ?? "none"}]\x1b[0m\r\n`,
          );
          break;
      }
    };
  }, []);

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    (async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");

      const term = new Terminal({
        allowProposedApi: true,
        cursorBlink: true,
        fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
        fontSize: 14,
        theme: {
          background: "#0d1117",
          foreground: "#e6edf3",
          cursor: "#e6edf3",
          selectionBackground: "#264f78",
          black: "#484f58",
          red: "#ff7b72",
          green: "#3fb950",
          yellow: "#d29922",
          blue: "#58a6ff",
          magenta: "#bc8cff",
          cyan: "#39c5cf",
          white: "#b1bac4",
          brightBlack: "#6e7681",
          brightRed: "#ffa198",
          brightGreen: "#56d364",
          brightYellow: "#e3b341",
          brightBlue: "#79c0ff",
          brightMagenta: "#d2a8ff",
          brightCyan: "#56d4dd",
          brightWhite: "#f0f6fc",
        },
      });
      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);

      if (termContainerRef.current) {
        term.open(termContainerRef.current);
        fitAddon.fit();
      }
      termRef.current = term;

      term.onData((data) => {
        if (wsRef.current?.readyState === WebSocket.OPEN) {
          wsRef.current.send(
            JSON.stringify({ type: "input", bytes: Array.from(new TextEncoder().encode(data)) }),
          );
        }
      });

      const ro = new ResizeObserver(() => {
        fitAddon.fit();
        if (wsRef.current?.readyState === WebSocket.OPEN && term.cols && term.rows) {
          wsRef.current.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
        }
      });
      if (termContainerRef.current) ro.observe(termContainerRef.current);

      openWs();

      cleanup = () => {
        wsRef.current?.close();
        ro.disconnect();
        term.dispose();
      };
    })();

    return () => {
      cleanup?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const reconnect = useCallback(() => {
    wsRef.current?.close();
    termRef.current?.clear();
    openWs();
  }, [openWs]);

  const interrupt = useCallback(() => {
    wsRef.current?.send(JSON.stringify({ type: "interrupt" }));
  }, []);

  const dot = status === "connected" ? "#3fb950" : status === "connecting" ? "#d29922" : "#ff7b72";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        background: "#0d1117",
        color: "#e6edf3",
      }}
    >
      {/* Toolbar */}
      <div
        style={{
          padding: "0.3rem 0.75rem",
          borderBottom: "1px solid #21262d",
          display: "flex",
          alignItems: "center",
          gap: "0.6rem",
          flexShrink: 0,
          background: "#161b22",
          fontSize: "0.75rem",
        }}
      >
        <span style={{ color: "#8b949e" }}>Terminal</span>
        <span
          style={{
            width: 7,
            height: 7,
            borderRadius: "50%",
            background: dot,
            display: "inline-block",
            flexShrink: 0,
          }}
        />
        <span style={{ color: dot }}>{status}</span>
        {sessionId && (
          <span style={{ color: "#484f58", fontFamily: "monospace", fontSize: "0.68rem" }}>
            {sessionId.slice(0, 8)}
          </span>
        )}
        <div style={{ marginLeft: "auto", display: "flex", gap: "0.4rem" }}>
          <TBtn onClick={interrupt}>⌃C</TBtn>
          <TBtn onClick={reconnect}>New session</TBtn>
        </div>
      </div>

      <div ref={termContainerRef} style={{ flex: 1, overflow: "hidden", padding: "4px 4px 0" }} />
    </div>
  );
}

function TBtn({ onClick, children }: { onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      style={{
        padding: "0.15rem 0.5rem",
        background: "#21262d",
        border: "1px solid #30363d",
        color: "#c9d1d9",
        borderRadius: "4px",
        cursor: "pointer",
        fontSize: "0.72rem",
        fontFamily: "inherit",
      }}
    >
      {children}
    </button>
  );
}
