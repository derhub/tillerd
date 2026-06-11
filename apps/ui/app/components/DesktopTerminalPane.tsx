import { useEffect, useRef, useState } from "react";
import "@xterm/xterm/css/xterm.css";

const TERM_THEME = {
  background: "#0d1117",
  foreground: "#e6edf3",
  cursor: "#e6edf3",
  black: "#0d1117",
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

export function DesktopTerminalPane(_props: {
  sessionId: string | null;
  cwd: string;
  onSessionStart?: (id: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<string>("connecting");

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | undefined;

    (async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");
      const { loadTerminalSurfaceTransport } = await import(
        "~/lib/transport/terminal-surface"
      );
      const { createTerminalSurfaceClient } = await import(
        "@tillerd/sdk/orchestrator"
      );

      if (cancelled) return;

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

      const transport = await loadTerminalSurfaceTransport();
      if (cancelled) {
        term.dispose();
        return;
      }

      const client = createTerminalSurfaceClient(transport);

      const surfaceId = await client.create(
        { cols: term.cols, rows: term.rows },
        (bytes) => term.write(bytes),
      );

      if (cancelled) {
        void client.detach(surfaceId);
        term.dispose();
        return;
      }

      if (!cancelled) setStatus("connected");

      term.onData((data) => {
        void client.input(surfaceId, new TextEncoder().encode(data));
      });

      const ro = new ResizeObserver(() => {
        fitAddon.fit();
        if (term.cols && term.rows) {
          void client.resize(surfaceId, term.cols, term.rows);
        }
      });
      if (containerRef.current) ro.observe(containerRef.current);

      // Filter status/exit events to this surface only.
      const unsubStatus = await client.onStatus((e) => {
        if (e.surfaceId === surfaceId && !cancelled) setStatus(e.status);
      });

      const unsubExit = await client.onExit((e) => {
        if (e.surfaceId === surfaceId && !cancelled) setStatus("exited");
      });

      cleanup = () => {
        ro.disconnect();
        unsubStatus();
        unsubExit();
        void client.detach(surfaceId);
        term.dispose();
      };
    })();

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, []);

  const dotColor =
    status === "connected"
      ? "#3fb950"
      : status === "exited"
        ? "#ff7b72"
        : "#8b949e";

  return (
    <div className="h-full w-full relative" style={{ background: "#0d1117" }}>
      <div
        ref={containerRef}
        className="h-full w-full"
        style={{ padding: "0.333rem 0.333rem 0" }}
      />
      <div
        style={{
          position: "absolute",
          top: "0.5rem",
          right: "0.75rem",
          display: "flex",
          alignItems: "center",
          gap: "0.375rem",
          pointerEvents: "none",
        }}
      >
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: dotColor,
            display: "inline-block",
          }}
        />
        <span style={{ color: "#8b949e", fontSize: 11 }}>{status}</span>
      </div>
    </div>
  );
}
