import { useQueryClient } from "@tanstack/react-query";
import React from "react";

import { scalarString } from "~/lib/json";
import "@xterm/xterm/css/xterm.css";

import { lazyFitAddon, lazyXterm } from "~/lib/lazy";
import { WS_BASE } from "~/lib/serverUrl";
import { SessionContext } from "~/lib/sessionContext";
import { getTerminalTheme } from "~/lib/settings/terminal-schemes";
import { useLiveTerminalTheme } from "~/lib/settings/useLiveTerminalTheme";
import { subscribe } from "~/lib/subscribe";

type Props = {
  sessionId: string | null;
  onSessionStart?: (id: string) => void;
};

async function bindTerminalPane(
  containerRef: React.RefObject<HTMLDivElement | null>,
  termRef: React.RefObject<import("@xterm/xterm").Terminal | null>,
  wsRef: React.RefObject<WebSocket | null>,
  coalesceBufRef: React.RefObject<Uint8Array[]>,
  coalesceTimerRef: React.RefObject<ReturnType<typeof setTimeout> | null>,
  openWs: (id: string | null) => void,
  sessionId: string | null,
  terminalTheme: ReturnType<typeof getTerminalTheme>,
): Promise<() => void> {
  const { Terminal } = await lazyXterm();
  const { FitAddon } = await lazyFitAddon();

  const term = new Terminal({
    allowProposedApi: true,
    cursorBlink: true,
    fontFamily: '"Geist Mono Variable", "Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
    fontSize: 13,
    theme: terminalTheme,
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

  openWs(sessionId);

  return () => {
    if (coalesceTimerRef.current !== null) {
      clearTimeout(coalesceTimerRef.current);
      coalesceTimerRef.current = null;
    }
    wsRef.current?.close();
    ro.disconnect();
    term.dispose();
    termRef.current = null;
  };
}

export function TerminalPane({ sessionId, onSessionStart }: Props) {
  const containerRef = React.useRef<HTMLDivElement>(null);
  const termRef = React.useRef<import("@xterm/xterm").Terminal | null>(null);
  const wsRef = React.useRef<WebSocket | null>(null);
  const openWsRef = React.useRef<((id: string | null) => void) | null>(null);
  const queryClient = useQueryClient();
  const { setStatus } = React.use(SessionContext);
  const [_connected, setConnected] = React.useState(false);
  const [crashedSessionId, setCrashedSessionId] = React.useState<string | null>(null);
  const coalesceBufRef = React.useRef<Uint8Array[]>([]);
  const coalesceTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);

  const openWs = React.useCallback(
    (id: string | null) => {
      if (!id) return; // never spawn bare -- user must trigger via useSpawnSession
      const url = `${WS_BASE}/ws/session?id=${id}`;
      const ws = new WebSocket(url);
      wsRef.current = ws;

      const flushCoalesce = () => {
        const chunks = coalesceBufRef.current;
        if (chunks.length === 0) return;
        const totalLen = chunks.reduce((s, c) => s + c.length, 0);
        const merged = new Uint8Array(totalLen);
        let offset = 0;
        for (const chunk of chunks) {
          merged.set(chunk, offset);
          offset += chunk.length;
        }
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
            const newId = scalarString(msg["sessionId"]);
            onSessionStart?.(newId);
            void queryClient.invalidateQueries({ queryKey: ["sessions"] });
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
            const status = scalarString(msg["status"]);
            setStatus(status);
            if (status === "crashed") {
              setCrashedSessionId(id);
            }
            break;
          }
          case "exit": {
            const qualifier = scalarString(msg["qualifier"], "unknown");
            const raw = msg["raw"] as Record<string, unknown> | undefined;
            const signalMeaning = raw?.["signalMeaning"] as string | undefined;
            const signalName = raw?.["signalName"] as string | undefined;
            const detail =
              signalMeaning && signalName
                ? `${signalName} — ${signalMeaning}`
                : raw?.["code"] != null
                  ? `code ${scalarString(raw["code"])}`
                  : qualifier;
            const color = qualifier === "ok" || qualifier === "stopped-by-request" ? "33" : "31";
            termRef.current?.write(
              `\r\n\x1b[${color}m[exited: ${qualifier}${qualifier !== detail ? ` — ${detail}` : ""}]\x1b[0m\r\n`,
            );
            void queryClient.invalidateQueries({ queryKey: ["sessions"] });
            break;
          }
        }
      };
    },
    [onSessionStart, queryClient, setStatus],
  );

  const terminalTheme = useLiveTerminalTheme(termRef);

  React.useEffect(() => {
    openWsRef.current = openWs;
    return subscribe(
      bindTerminalPane(
        containerRef,
        termRef,
        wsRef,
        coalesceBufRef,
        coalesceTimerRef,
        openWs,
        sessionId,
        terminalTheme,
      ),
    );
    // sessionId/terminalTheme intentionally excluded: reconnect is handled via key prop at call
    // site; the scheme effect above pushes live theme updates without rebinding the socket.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const _interrupt = React.useCallback(() => {
    wsRef.current?.send(JSON.stringify({ type: "interrupt" }));
  }, []);

  const _reconnect = React.useCallback(() => {
    wsRef.current?.close();
    termRef.current?.clear();
    openWsRef.current?.(sessionId);
  }, [sessionId]);

  const handleRecover = React.useCallback(() => {
    if (!crashedSessionId || wsRef.current?.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "spawn", resume: crashedSessionId }));
    setCrashedSessionId(null);
  }, [crashedSessionId]);

  const handleDismiss = React.useCallback(() => {
    if (!crashedSessionId || wsRef.current?.readyState !== WebSocket.OPEN) return;
    wsRef.current.send(JSON.stringify({ type: "stop" }));
    setCrashedSessionId(null);
  }, [crashedSessionId]);

  return (
    <div className="h-full w-full relative" style={{ background: terminalTheme.background }}>
      <div
        ref={containerRef}
        className="h-full w-full"
        style={{ padding: "0.333rem 0.333rem 0" }}
      />
      {crashedSessionId && (
        // Surface failure overlay (ui-terminal-pane spec): terminal-token styled, distinct from
        // the service-health indicator.
        <div
          data-testid="terminal-failure-overlay"
          className="absolute bottom-4 left-1/2 -translate-x-1/2 flex items-center gap-2 rounded-md border border-terminal-border bg-terminal-surface px-4 py-3 text-[0.917rem] text-terminal-fg"
        >
          <span className="text-terminal-error">Session ended unexpectedly —</span>
          <button
            onClick={handleRecover}
            className="rounded-sm bg-terminal-success px-3 py-1 text-terminal-fg transition-colors duration-[var(--motion-fast)] ease-standard hover:brightness-110"
          >
            Resume
          </button>
          <button
            onClick={handleDismiss}
            className="rounded-sm border border-terminal-border px-3 py-1 text-terminal-muted transition-colors duration-[var(--motion-fast)] ease-standard hover:text-terminal-fg"
          >
            Dismiss
          </button>
        </div>
      )}
    </div>
  );
}
