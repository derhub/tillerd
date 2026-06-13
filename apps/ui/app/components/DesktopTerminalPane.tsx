import { useEffect, useRef, useState } from "react";
import type { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";

import { useSettingsContext } from "~/lib/settings/context";
import { TERMINAL_SCHEME_KEY } from "~/lib/settings/keys";
import { DEFAULT_TERMINAL_SCHEME, getTerminalTheme } from "~/lib/settings/terminal-schemes";
import { useStringSetting } from "~/lib/settings/use-settings";

export function DesktopTerminalPane(_props: {
  sessionId: string | null;
  placement: string;
  cwd: string;
  onSessionStart?: (id: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [status, setStatus] = useState<string>("connecting");
  // Exposed as `data-surface-id` so a session's surface is observable (e.g. the desktop e2e asserts
  // two sessions get two distinct surfaces).
  const [surfaceId, setSurfaceId] = useState<string | null>(null);

  // Terminal color scheme: applied at creation and updated live without recreating the PTY.
  const source = useSettingsContext();
  const { value: scheme } = useStringSetting(source, TERMINAL_SCHEME_KEY, DEFAULT_TERMINAL_SCHEME);
  const termRef = useRef<Terminal | null>(null);
  const schemeRef = useRef(scheme);
  schemeRef.current = scheme;

  useEffect(() => {
    const term = termRef.current;
    if (term) term.options.theme = getTerminalTheme(scheme);
  }, [scheme]);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | undefined;

    (async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");
      const { loadTerminalSurfaceTransport } = await import("~/lib/transport/terminal-surface");
      const { createTerminalSurfaceClient } = await import("@tillerd/sdk/orchestrator");

      if (cancelled) return;

      const term = new Terminal({
        allowProposedApi: true,
        cursorBlink: true,
        fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
        fontSize: 13,
        theme: getTerminalTheme(schemeRef.current),
      });
      termRef.current = term;
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
        {
          sessionId: _props.sessionId ?? "",
          placement: _props.placement,
          cols: term.cols,
          rows: term.rows,
        },
        (bytes) => term.write(bytes),
      );

      if (cancelled) {
        void client.detach(surfaceId);
        term.dispose();
        return;
      }

      if (!cancelled) {
        setSurfaceId(surfaceId);
        setStatus("connected");
      }

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
        termRef.current = null;
      };
    })();

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, []);

  const dotColor = status === "connected" ? "#3fb950" : status === "exited" ? "#ff7b72" : "#8b949e";

  return (
    <div
      className="h-full w-full relative"
      style={{ background: getTerminalTheme(scheme).background }}
      data-surface-id={surfaceId ?? undefined}
    >
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
