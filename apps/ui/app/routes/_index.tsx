import { useEffect, useRef, useState, useCallback } from "react";
import type { Route } from "./+types/_index";
import type { SessionStatus, ContentEvent } from "@athing/sdk";

export const meta: Route.MetaFunction = () => [
  { title: "a-thing" },
  { name: "description", content: "Agent session terminal" },
];

const SERVER = "localhost:3000";
const WS_URL = `ws://${SERVER}/ws/session`;

interface ContentItem {
  id: number;
  event: ContentEvent;
}

let _idCounter = 0;

export default function TerminalPage() {
  const termContainerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<import("@xterm/xterm").Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const [status, setStatus] = useState<SessionStatus | null>(null);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [content, setContent] = useState<ContentItem[]>([]);
  const [connected, setConnected] = useState(false);
  const [promptText, setPromptText] = useState("");

  useEffect(() => {
    let term: import("@xterm/xterm").Terminal;
    let fitAddon: import("@xterm/addon-fit").FitAddon;
    let ws: WebSocket;

    (async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");

      term = new Terminal({ allowProposedApi: true });
      fitAddon = new FitAddon();
      term.loadAddon(fitAddon);

      if (termContainerRef.current) {
        term.open(termContainerRef.current);
        fitAddon.fit();
      }
      termRef.current = term;

      ws = new WebSocket(WS_URL);
      wsRef.current = ws;

      ws.binaryType = "arraybuffer";

      ws.onopen = () => setConnected(true);
      ws.onclose = () => {
        setConnected(false);
        setStatus(null);
      };

      ws.onmessage = (e) => {
        const msg = JSON.parse(e.data as string) as Record<string, unknown>;
        switch (msg["type"]) {
          case "session_start":
            setSessionId(String(msg["sessionId"] ?? ""));
            break;
          case "data": {
            const arr = new Uint8Array(msg["bytes"] as number[]);
            term.write(arr);
            break;
          }
          case "status":
            setStatus(msg["status"] as SessionStatus);
            break;
          case "content":
            setContent((prev) => [
              ...prev.slice(-99),
              { id: ++_idCounter, event: msg["event"] as ContentEvent },
            ]);
            break;
        }
      };

      term.onData((data) => {
        if (ws.readyState === WebSocket.OPEN) {
          const bytes = Array.from(new TextEncoder().encode(data));
          ws.send(JSON.stringify({ type: "input", bytes }));
        }
      });

      const resizeObserver = new ResizeObserver(() => {
        fitAddon.fit();
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: "resize", cols: term.cols, rows: term.rows }));
        }
      });
      if (termContainerRef.current) {
        resizeObserver.observe(termContainerRef.current);
      }
    })();

    return () => {
      ws?.close();
      term?.dispose();
    };
  }, []);

  const sendPrompt = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN && promptText.trim()) {
      wsRef.current.send(JSON.stringify({ type: "send", text: promptText }));
      setPromptText("");
    }
  }, [promptText]);

  const interrupt = useCallback(() => {
    wsRef.current?.send(JSON.stringify({ type: "interrupt" }));
  }, []);

  const statusColor: Record<string, string> = {
    IDLE: "#22c55e",
    WORKING: "#f59e0b",
    WAITING_INPUT: "#3b82f6",
    DONE: "#6b7280",
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100vh",
        background: "#0d1117",
        color: "#e6edf3",
      }}
    >
      <div
        style={{
          padding: "0.5rem 1rem",
          borderBottom: "1px solid #30363d",
          display: "flex",
          alignItems: "center",
          gap: "1rem",
        }}
      >
        <strong>a-thing</strong>
        {sessionId && (
          <span style={{ fontSize: "0.75rem", color: "#8b949e" }}>
            session: {sessionId.slice(0, 8)}
          </span>
        )}
        {status && (
          <span
            style={{
              fontSize: "0.75rem",
              fontWeight: 600,
              color: statusColor[status] ?? "#e6edf3",
            }}
          >
            {status}
          </span>
        )}
        {!connected && <span style={{ fontSize: "0.75rem", color: "#f87171" }}>disconnected</span>}
        <button
          onClick={interrupt}
          style={{
            marginLeft: "auto",
            padding: "0.25rem 0.75rem",
            background: "#21262d",
            border: "1px solid #30363d",
            color: "#e6edf3",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          Interrupt
        </button>
      </div>

      <div style={{ display: "flex", flex: 1, overflow: "hidden" }}>
        <div ref={termContainerRef} style={{ flex: 1, overflow: "hidden" }} />

        <div
          style={{
            width: "280px",
            borderLeft: "1px solid #30363d",
            display: "flex",
            flexDirection: "column",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              padding: "0.5rem 1rem",
              borderBottom: "1px solid #30363d",
              fontSize: "0.75rem",
              color: "#8b949e",
            }}
          >
            CONTENT
          </div>
          <div style={{ flex: 1, overflowY: "auto", padding: "0.5rem" }}>
            {content.map((item) => (
              <ContentCard key={item.id} event={item.event} />
            ))}
          </div>
        </div>
      </div>

      <div
        style={{
          padding: "0.5rem",
          borderTop: "1px solid #30363d",
          display: "flex",
          gap: "0.5rem",
        }}
      >
        <input
          value={promptText}
          onChange={(e) => setPromptText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              sendPrompt();
            }
          }}
          placeholder="Send a prompt…"
          style={{
            flex: 1,
            padding: "0.5rem",
            background: "#161b22",
            border: "1px solid #30363d",
            color: "#e6edf3",
            borderRadius: "4px",
          }}
        />
        <button
          onClick={sendPrompt}
          style={{
            padding: "0.5rem 1rem",
            background: "#1f6feb",
            border: "none",
            color: "#fff",
            borderRadius: "4px",
            cursor: "pointer",
          }}
        >
          Send
        </button>
      </div>
    </div>
  );
}

function ContentCard({ event }: { event: ContentEvent }) {
  if (event.kind === "tool_use") {
    return (
      <div
        style={{
          marginBottom: "0.5rem",
          padding: "0.5rem",
          background: "#161b22",
          borderRadius: "4px",
          fontSize: "0.7rem",
        }}
      >
        <div style={{ color: "#79c0ff" }}>tool: {event.toolName}</div>
        <pre style={{ margin: 0, color: "#8b949e", overflow: "hidden", maxHeight: "4rem" }}>
          {JSON.stringify(event.toolInput, null, 1).slice(0, 200)}
        </pre>
      </div>
    );
  }
  if (event.kind === "edit") {
    return (
      <div
        style={{
          marginBottom: "0.5rem",
          padding: "0.5rem",
          background: "#161b22",
          borderRadius: "4px",
          fontSize: "0.7rem",
        }}
      >
        <div style={{ color: "#7ee787" }}>edit: {event.filePath.split("/").pop()}</div>
      </div>
    );
  }
  if (event.kind === "usage") {
    return (
      <div
        style={{
          marginBottom: "0.5rem",
          padding: "0.5rem",
          background: "#161b22",
          borderRadius: "4px",
          fontSize: "0.7rem",
          color: "#8b949e",
        }}
      >
        in: {event.inputTokens} out: {event.outputTokens}
        {event.costUsd != null && ` $${event.costUsd.toFixed(4)}`}
      </div>
    );
  }
  return null;
}
