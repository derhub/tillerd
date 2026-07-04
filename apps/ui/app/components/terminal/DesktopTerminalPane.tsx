import type { SurfaceChannelHandle } from "@tillerd/client-bindings";
import type { IDisposable, Terminal } from "@xterm/xterm";

import { useMutation } from "@tanstack/react-query";
import { command, runCommand, surfaceChannel } from "@tillerd/client-bindings";
import "@xterm/xterm/css/xterm.css";
import React from "react";

import { lazyFitAddon, lazyXterm } from "~/lib/lazy";
import { getTerminalTheme } from "~/lib/settings/terminal-schemes";
import { useLiveTerminalTheme } from "~/lib/settings/useLiveTerminalTheme";
import { subscribe as bridgeSubscribe } from "~/lib/subscribe";

// Creates the xterm.js Terminal + DOM canvas exactly once per mount and hands it to the caller via
// termRef/setTerminalReady before resolving -- this survives a placement swap (panel-placement-swap
// spec: "no remount of the xterm nodes"); only the data channel bound to it is torn down and
// rebuilt by bindChannel below. A plain async helper (not a hook/component), so it -- not the
// effect that calls it -- owns the await chain; the effect only calls subscribe() on the result.
async function mountTerminal(
  abort: { cancelled: boolean },
  containerRef: React.RefObject<HTMLDivElement | null>,
  terminalTheme: ReturnType<typeof getTerminalTheme>,
  termRef: React.RefObject<Terminal | null>,
  setTerminalReady: (ready: boolean) => void,
  detachOnUnmount: boolean,
  surfaceIdRef: React.RefObject<string | null>,
  detachSurface: (surfaceId: string) => void,
): Promise<() => void> {
  const { Terminal } = await lazyXterm();
  const { FitAddon } = await lazyFitAddon();
  if (abort.cancelled) return () => {};

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
  if (abort.cancelled) {
    term.dispose();
    return () => {};
  }

  const ro = new ResizeObserver(() => fitAddon.fit());
  if (containerRef.current) ro.observe(containerRef.current);

  termRef.current = term;
  setTerminalReady(true);

  return () => {
    ro.disconnect();
    if (detachOnUnmount && surfaceIdRef.current) {
      detachSurface(surfaceIdRef.current);
    }
    term.dispose();
    termRef.current = null;
  };
}

// Resolves the surface currently behind (session, placement) and binds its byte/status channel to
// the already-open Terminal. Reruns whenever placement or reloadKey change (a swap landed) without
// touching the Terminal/DOM -- only the channel and the onData wiring are replaced.
async function bindChannel(
  abort: { cancelled: boolean },
  term: Terminal,
  sessionId: string,
  placement: string,
  setSurfaceId: (id: string) => void,
  setStatus: (s: string) => void,
): Promise<() => void> {
  const view = await runCommand("surfaceResolveOrSpawn", {
    session: sessionId,
    placement,
    cols: term.cols,
    rows: term.rows,
    cwd: null,
  });
  const surfaceId = view.id;
  if (abort.cancelled) return () => {};

  const handle: SurfaceChannelHandle = await surfaceChannel({ surfaceId }, (event) => {
    if (abort.cancelled) return;
    switch (event.kind) {
      case "bytes":
        term.write(event.value);
        break;
      case "status":
        setStatus(event.value);
        break;
      case "exit":
        setStatus("exited");
        break;
      case "error":
        setStatus("error");
        break;
    }
  });
  if (abort.cancelled) {
    void handle.close();
    return () => {};
  }

  setSurfaceId(surfaceId);
  setStatus("connected");

  const encoder = new TextEncoder();
  const onData: IDisposable = term.onData((data) => {
    void handle.send({ kind: "input", bytes: Array.from(encoder.encode(data)) });
  });
  const onResize: IDisposable = term.onResize(({ cols, rows }) => {
    void handle.send({ kind: "resize", cols, rows });
  });

  return () => {
    onData.dispose();
    onResize.dispose();
    void handle.close();
  };
}

export function DesktopTerminalPane(_props: {
  sessionId: string | null;
  placement: string;
  cwd: string;
  onSessionStart?: (id: string) => void;
  detachOnUnmount?: boolean;
  // Bumped by a successful placement swap to force a channel rebind without recreating the
  // Terminal (see bindChannel above).
  reloadKey?: number;
}) {
  const detachOnUnmount = _props.detachOnUnmount ?? true;
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [status, setStatus] = React.useState<string>("connecting");
  const [surfaceId, setSurfaceId] = React.useState<string | null>(null);
  const surfaceIdRef = React.useRef<string | null>(null);
  surfaceIdRef.current = surfaceId;

  const termRef = React.useRef<Terminal | null>(null);
  const terminalTheme = useLiveTerminalTheme(termRef);
  const [terminalReady, setTerminalReady] = React.useState(false);

  const detach = useMutation(command("surfaceDetach"));
  const detachRef = React.useRef(detach.mutateAsync);
  detachRef.current = detach.mutateAsync;

  // Mount the Terminal once. Cleanup only fires on a true unmount (deps: []), which is where
  // detachOnUnmount applies -- a placement swap never runs this cleanup.
  React.useEffect(() => {
    const abort = { cancelled: false };
    const unsub = bridgeSubscribe(
      mountTerminal(
        abort,
        containerRef,
        terminalTheme,
        termRef,
        setTerminalReady,
        detachOnUnmount,
        surfaceIdRef,
        (id) => void detachRef.current({ id }),
      ),
    );
    return () => {
      abort.cancelled = true;
      unsub();
    };
    // Terminal creation happens once per mount; theme updates are pushed via the effect above,
    // and detachOnUnmount/reloadKey are read through refs/the effect below respectively.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Bind (or rebind) the data channel whenever the placement's backing surface may have changed.
  React.useEffect(() => {
    if (!terminalReady || !termRef.current || !_props.sessionId) return () => {};
    const abort = { cancelled: false };
    const unsub = bridgeSubscribe(
      bindChannel(
        abort,
        termRef.current,
        _props.sessionId,
        _props.placement,
        setSurfaceId,
        setStatus,
      ),
    );
    return () => {
      abort.cancelled = true;
      unsub();
    };
  }, [terminalReady, _props.sessionId, _props.placement, _props.reloadKey]);

  const dotColorClass =
    status === "connected"
      ? "bg-terminal-success"
      : status === "exited"
        ? "bg-terminal-error"
        : "bg-terminal-muted";

  return (
    <div
      className="h-full w-full relative"
      style={{ background: terminalTheme.background }}
      data-surface-id={surfaceId ?? undefined}
    >
      <div
        ref={containerRef}
        className="h-full w-full"
        style={{ padding: "0.333rem 0.333rem 0" }}
      />
      <div className="absolute top-2 right-3 flex items-center gap-1.5 pointer-events-none">
        <span className={`w-2 h-2 rounded-full inline-block ${dotColorClass}`} />
        <span className="text-terminal-muted text-[0.917rem]">{status}</span>
      </div>
    </div>
  );
}
