import type { ChannelHandle } from "@tillerd/client-bindings";
import type { Terminal } from "@xterm/xterm";

import { useMutation } from "@tanstack/react-query";
import { command, openSurfaceChannel, subscribe } from "@tillerd/client-bindings";
import "@xterm/xterm/css/xterm.css";
import React from "react";

import { lazyFitAddon, lazyXterm } from "~/lib/lazy";
import { useGlobalSetting } from "~/lib/settings/context";
import { TERMINAL_SCHEME_KEY } from "~/lib/settings/keys";
import { DEFAULT_TERMINAL_SCHEME, getTerminalTheme } from "~/lib/settings/terminal-schemes";
import { subscribe as bridgeSubscribe } from "~/lib/subscribe";

// Async setup extracted so the component effect stays non-async.
// The abort object is mutated by the effect cleanup to signal cancellation at each await point.
async function bindDesktopTerminal(
  abort: { cancelled: boolean },
  containerRef: React.RefObject<HTMLDivElement | null>,
  termRef: React.RefObject<Terminal | null>,
  terminalTheme: ReturnType<typeof getTerminalTheme>,
  sessionId: string,
  placement: string,
  setSurfaceId: (id: string) => void,
  setStatus: (s: string) => void,
  detachOnUnmount: boolean,
  detachSurface: (surfaceId: string) => void,
): Promise<() => void> {
  const { Terminal } = await lazyXterm();
  const { FitAddon } = await lazyFitAddon();

  if (abort.cancelled) return () => {};

  const term = new Terminal({
    allowProposedApi: true,
    cursorBlink: true,
    fontFamily: '"Cascadia Code", "Fira Code", "JetBrains Mono", monospace',
    fontSize: 13,
    theme: terminalTheme,
  });
  termRef.current = term;
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

  const handle: ChannelHandle = await openSurfaceChannel({
    sessionId,
    placement,
    cols: term.cols,
    rows: term.rows,
    cwd: null,
  });
  handle.onmessage = (bytes) => term.write(new Uint8Array(bytes));
  const surfaceId = handle.surfaceId;

  if (abort.cancelled) {
    detachSurface(surfaceId);
    term.dispose();
    return () => {};
  }

  setSurfaceId(surfaceId);
  setStatus("connected");

  const encoder = new TextEncoder();
  term.onData((data) => {
    void handle.send(encoder.encode(data));
  });

  const ro = new ResizeObserver(() => {
    fitAddon.fit();
    if (term.cols && term.rows) {
      void handle.resize(term.cols, term.rows);
    }
  });
  if (containerRef.current) ro.observe(containerRef.current);

  const unsubStatus = await subscribe("surfaceStatus").listen((e) => {
    if (e.payload.surfaceId === surfaceId && !abort.cancelled) setStatus(e.payload.status);
  });

  const unsubExit = await subscribe("surfaceExit").listen((e) => {
    if (e.payload.surfaceId === surfaceId && !abort.cancelled) setStatus("exited");
  });

  return () => {
    ro.disconnect();
    unsubStatus();
    unsubExit();
    // Stop local delivery without tearing down the backend PTY; detach (when owning) keeps the
    // surface alive for the parent's revisit path. A detached child window skips detach to avoid a
    // channel-remove race.
    handle.onmessage = () => {};
    if (detachOnUnmount) detachSurface(surfaceId);
    term.dispose();
    termRef.current = null;
  };
}

export function DesktopTerminalPane(_props: {
  sessionId: string | null;
  placement: string;
  cwd: string;
  onSessionStart?: (id: string) => void;
  // Detach the surface proxy on unmount (default). A detached child window passes `false` so closing
  // it leaves the live PTY for the parent's revisit path to re-bind -- avoids a channel-remove race.
  detachOnUnmount?: boolean;
}) {
  const detachOnUnmount = _props.detachOnUnmount ?? true;
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [status, setStatus] = React.useState<string>("connecting");
  // Exposed as `data-surface-id` so a session's surface is observable (e.g. the desktop e2e asserts
  // two sessions get two distinct surfaces).
  const [surfaceId, setSurfaceId] = React.useState<string | null>(null);

  const { value: scheme } = useGlobalSetting(TERMINAL_SCHEME_KEY, DEFAULT_TERMINAL_SCHEME);
  const terminalTheme = getTerminalTheme(scheme);
  const termRef = React.useRef<Terminal | null>(null);

  const detach = useMutation(command("surfaceDetach"));
  const detachRef = React.useRef(detach.mutateAsync);
  detachRef.current = detach.mutateAsync;

  React.useEffect(() => {
    const term = termRef.current;
    if (term) term.options.theme = terminalTheme;
  }, [terminalTheme]);

  React.useEffect(() => {
    const abort = { cancelled: false };
    const unsub = bridgeSubscribe(
      bindDesktopTerminal(
        abort,
        containerRef,
        termRef,
        terminalTheme,
        _props.sessionId ?? "",
        _props.placement,
        (id) => setSurfaceId(id),
        setStatus,
        detachOnUnmount,
        (id) => void detachRef.current({ surfaceId: id }),
      ),
    );
    return () => {
      abort.cancelled = true;
      unsub();
    };
  }, []);

  const dotColor = status === "connected" ? "#3fb950" : status === "exited" ? "#ff7b72" : "#8b949e";

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
