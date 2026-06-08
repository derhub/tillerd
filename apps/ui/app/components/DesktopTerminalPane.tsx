import { useEffect, useRef } from "react";
import type { AgentSession } from "@athing/sdk";
import { bindSessionToTerminal } from "~/lib/transport";
import { useDesktopHost } from "~/lib/useDesktopHost";
import "@xterm/xterm/css/xterm.css";

const TERM_THEME = {
  background: "#0d1117",
  foreground: "#e6edf3",
  cursor: "#e6edf3",
  selectionBackground: "#264f78",
};

/**
 * Desktop terminal: drives an xterm directly from an engine `AgentSession` over the native
 * transport — no web server. A null `sessionId` starts a new session; an id reconnects.
 */
export function DesktopTerminalPane({
  sessionId,
  cwd,
  onSessionStart,
}: {
  sessionId: string | null;
  cwd: string;
  onSessionStart?: (id: string) => void;
}) {
  const containerRef = useRef<HTMLDivElement>(null);
  const host = useDesktopHost();

  useEffect(() => {
    if (host.status !== "ready") return;
    let cleanup: (() => void) | undefined;
    let disposed = false;

    void (async () => {
      const { Terminal } = await import("@xterm/xterm");
      const { FitAddon } = await import("@xterm/addon-fit");

      const term = new Terminal({
        allowProposedApi: true,
        cursorBlink: true,
        fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
        fontSize: 13,
        theme: TERM_THEME,
      });
      const fit = new FitAddon();
      term.loadAddon(fit);
      if (containerRef.current) {
        term.open(containerRef.current);
        fit.fit();
      }

      const { engine, agent } = host.host;
      let session: AgentSession;
      try {
        session = sessionId
          ? await engine.reconnect(sessionId, agent, { cwd })
          : await engine.start(agent, { cwd, cols: term.cols, rows: term.rows });
      } catch (err) {
        term.write(`\r\n\x1b[31m[engine error: ${(err as Error).message}]\x1b[0m\r\n`);
        cleanup = () => term.dispose();
        return;
      }
      if (!sessionId) onSessionStart?.(session.sessionId);

      const unbind = bindSessionToTerminal(session, term);
      const offExit = session.onExit((e) =>
        term.write(`\r\n\x1b[33m[exited: ${e.qualifier}]\x1b[0m\r\n`),
      );
      const ro = new ResizeObserver(() => {
        fit.fit();
        if (term.cols && term.rows) session.resize(term.cols, term.rows);
      });
      if (containerRef.current) ro.observe(containerRef.current);

      if (disposed) {
        unbind();
        offExit();
        ro.disconnect();
        term.dispose();
        return;
      }
      cleanup = () => {
        unbind();
        offExit();
        ro.disconnect();
        term.dispose();
      };
    })();

    return () => {
      disposed = true;
      cleanup?.();
    };
    // session identity is keyed by the route; re-run only when the host becomes ready
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [host.status]);

  return (
    <div className="h-full w-full relative" style={{ background: "#0d1117" }}>
      <div
        ref={containerRef}
        className="h-full w-full"
        style={{ padding: "0.333rem 0.333rem 0" }}
      />
    </div>
  );
}
