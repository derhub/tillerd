import { useEffect, useRef, useCallback, useState } from "react";
import { useRevalidator } from "react-router";
import { use } from "react";
import { SessionContext } from "~/lib/sessionContext";
import "@xterm/xterm/css/xterm.css";

const WS_BASE = `ws://${typeof window !== "undefined" ? window.location.host : "localhost"}`;

type Props = {
  sessionId: string | null;
  onSessionStart?: (id: string) => void;
};

const TERM_THEME = {
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
};

export function TerminalPane({ sessionId, onSessionStart }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<import("@xterm/xterm").Terminal | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const openWsRef = useRef<((id: string | null) => void) | null>(null);
  const revalidator = useRevalidator();
  const { setStatus } = use(SessionContext);
  const [_connected, setConnected] = useState(false);
  const [crashedSessionId, setCrashedSessionId] = useState<string | null>(null);
  const coalesceBufRef = useRef<Uint8Array[]>([]);
  const coalesceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const openWs = useCallback(
    (id: string | null) => {
      if (!id) return; // never spawn bare — user must trigger via useSpawnSession
      const url = `${WS_BASE}/ws/session?id=${id}`;
      const ws = new WebSocket(url);
      wsRef.current = ws;

      const flushCoalesce = () => {
        const chunks = coalesceBufRef.current;
        if (chunks.length === 0) return;
        const totalLen = chunks.reduce((s, c) => s + c.length, 0);
        const merged = new Uint8Array(totalLen);
        let offset = 0;
        for (const chunk of chunks) { merged.set(chunk, offset); offset += chunk.length; }
        coalesceBufRef.current = [];
        coalesceTimerRef.current = null;
        termRef.current?.write(merged);
      };

      ws.onopen = () => {
        setConnected(true);
      };
      ws.onclose = () => {
        if (coalesceTimerRef.current !== null) {
          clearTimeout(coalesceTimerRef.current);
          flushCoalesce();
        }
        setConnected(false);
        termRef.current?.write("\r\n\x1b[31m[disconnected]\x1b[0m\r\n");
      };
      ws.onerror = () => {
        termRef.current?.write("\r\n\x1b[31m[connection error]\x1b[0m\r\n");
      };
      ws.onmessage = (e) => {
        const msg = JSON.parse(e.data as string) as Record<string, unknown>;
        switch (msg["type"]) {
          case "session_start": {
            const newId = String(msg["sessionId"] ?? "");
            onSessionStart?.(newId);
            revalidator.revalidate();
            break;
          }
          case "session_resume":
            break;
          case "data": {
            const arr = new Uint8Array(msg["bytes"] as number[]);
            coalesceBufRef.current.push(arr);
            const totalLen = coalesceBufRef.current.reduce((s, c) => s + c.length, 0);
            if (totalLen >= 4096) {
              if (coalesceTimerRef.current !== null) clearTimeout(coalesceTimerRef.current);
              flushCoalesce();
            } else if (coalesceTimerRef.current === null) {
              coalesceTimerRef.current = setTimeout(flushCoalesce, 8);
            }
            break;
          }
          case "status": {
            const status = String(msg["status"] ?? "");
            setStatus(status);
            if (status === "crashed") {
              setCrashedSessionId(id);
            }
            break;
          }
          case "exit": {
            const qualifier = String(msg["qualifier"] ?? "unknown");
            const raw = msg["raw"] as Record<string, unknown> | undefined;
            const signalMeaning = raw?.["signalMeaning"] as string | undefined;
            const signalName = raw?.["signalName"] as string | undefined;
            const detail = signalMeaning && signalName
              ? `${signalName} — ${signalMeaning}`
              : raw?.["code"] != null
              ? `code ${String(raw["code"])}`
              : qualifier;
            const color = qualifier === "ok" || qualifier === "stopped-by-request" ? "33" : "31";
            termRef.current?.write(
              `\r\n\x1b[${color}m[exited: ${qualifier}${qualifier !== detail ? ` — ${detail}` : ""}]\x1b[0m\r\n`,
            );
            revalidator.revalidate();
            break;
          }
        }
      };
    },
    [onSessionStart, revalidator, setStatus],
  );

  useEffect(() => {
    let cleanup: (() => void) | undefined;

    (async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");

      const term = new Terminal({
        allowProposedApi: true,
        cursorBlink: true,
        fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
        fontSize: 13,
        theme: TERM_THEME,
      });
      const fitAddon = new FitAddon();
      term.loadAddon(fitAddon);

      if (containerRef.current) {
        term.open(containerRef.current);
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
      if (containerRef.current) ro.observe(containerRef.current);

      openWsRef.current = openWs;
      openWs(sessionId);

      cleanup = () => {
        if (coalesceTimerRef.current !== null) {
          clearTimeout(coalesceTimerRef.current);
          coalesceTimerRef.current = null;
        }
        wsRef.current?.close();
        ro.disconnect();
        term.dispose();
        termRef.current = null;
      };
    })();

    return () => cleanup?.();
    // sessionId intentionally excluded: reconnect is handled via key prop at call site
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const _interrupt = useCallback(() => {
    wsRef.current?.send(JSON.stringify({ type: "interrupt" }));
  }, []);

  const _reconnect = useCallback(() => {
    wsRef.current?.close();
    termRef.current?.clear();
    openWsRef.current?.(sessionId);
  }, [sessionId]);

  const handleRecover = useCallback(() => {
    if (!crashedSessionId || wsRef.current?.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "spawn", resume: crashedSessionId }));
    setCrashedSessionId(null);
  }, [crashedSessionId]);

  const handleDismiss = useCallback(() => {
    if (!crashedSessionId || wsRef.current?.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "stop" }));
    setCrashedSessionId(null);
  }, [crashedSessionId]);

  return (
    <div className="h-full w-full relative" style={{ background: "#0d1117" }}>
      <div
        ref={containerRef}
        className="h-full w-full"
        style={{ padding: "0.333rem 0.333rem 0" }}
      />
      {crashedSessionId && (
        <div
          style={{
            position: "absolute",
            bottom: "1rem",
            left: "50%",
            transform: "translateX(-50%)",
            background: "#21262d",
            border: "1px solid #30363d",
            borderRadius: "6px",
            padding: "0.75rem 1rem",
            display: "flex",
            gap: "0.5rem",
            alignItems: "center",
            color: "#e6edf3",
            fontSize: "13px",
          }}
        >
          <span style={{ color: "#ff7b72" }}>Session ended unexpectedly —</span>
          <button
            onClick={handleRecover}
            style={{ background: "#238636", border: "none", borderRadius: "4px", color: "#fff", padding: "0.25rem 0.75rem", cursor: "pointer" }}
          >
            Resume
          </button>
          <button
            onClick={handleDismiss}
            style={{ background: "transparent", border: "1px solid #30363d", borderRadius: "4px", color: "#8b949e", padding: "0.25rem 0.75rem", cursor: "pointer" }}
          >
            Dismiss
          </button>
        </div>
      )}
    </div>
  );
}
