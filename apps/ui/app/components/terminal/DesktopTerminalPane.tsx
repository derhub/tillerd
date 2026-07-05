import type { SurfaceChannelHandle } from "@tillerd/client-bindings";
import type { FitAddon } from "@xterm/addon-fit";
import type { IDisposable, Terminal } from "@xterm/xterm";

import { useMutation } from "@tanstack/react-query";
import { command, runCommand, surfaceChannel } from "@tillerd/client-bindings";
import "@xterm/xterm/css/xterm.css";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { TerminalFailureOverlay } from "~/components/terminal/TerminalFailureOverlay";
import { lazyFitAddon, lazyXterm } from "~/lib/lazy";
import { getTerminalTheme } from "~/lib/settings/terminal-schemes";
import { useLiveTerminalTheme } from "~/lib/settings/useLiveTerminalTheme";
import {
  useLiveTerminalTypography,
  type TerminalTypography,
} from "~/lib/settings/useLiveTerminalTypography";
import { subscribe as bridgeSubscribe } from "~/lib/subscribe";

import { useTerminalPaneExtras } from "./useTerminalPaneExtras";

// Exit qualifiers the runtime treats as a clean stop (exit-classification contract): no failure
// overlay for these, matching the surface-status write in surface_channel.rs's exit_status().
const CLEAN_EXIT_QUALIFIERS = new Set(["ok", "stopped-by-request"]);

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
  fitAddonRef: React.RefObject<FitAddon | null>,
  setTerminalReady: (ready: boolean) => void,
  detachOnUnmount: boolean,
  surfaceIdRef: React.RefObject<string | null>,
  detachSurface: (surfaceId: string) => void,
  typography: TerminalTypography,
): Promise<() => void> {
  const { Terminal } = await lazyXterm();
  const { FitAddon } = await lazyFitAddon();
  if (abort.cancelled) return () => {};

  // Seed construction with the user's persisted typography so the first paint uses their
  // font/size instead of the defaults; useTerminalPaneExtras re-applies the live values on
  // attach and on every subsequent change (this just removes the initial reflow/refit flash).
  const term = new Terminal({
    allowProposedApi: true,
    cursorBlink: typography.cursorBlink,
    cursorStyle: typography.cursorStyle,
    fontFamily: typography.fontFamily,
    fontSize: typography.fontSize,
    lineHeight: typography.lineHeight,
    scrollback: typography.scrollback,
    theme: terminalTheme,
  });
  const fitAddon = new FitAddon();
  term.loadAddon(fitAddon);
  fitAddonRef.current = fitAddon;

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
    fitAddonRef.current = null;
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
  sendInputRef: React.RefObject<((bytes: number[]) => void) | null>,
  setFailureReason: (reason: string | null) => void,
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
        if (!CLEAN_EXIT_QUALIFIERS.has(event.value)) {
          setFailureReason(`Session exited unexpectedly (${event.value})`);
        }
        break;
      case "error":
        setStatus("error");
        setFailureReason(`Terminal error: ${event.value}`);
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
  // Publish the input path so the extras layer (path drop) can write to this surface's PTY.
  sendInputRef.current = (bytes) => void handle.send({ kind: "input", bytes });

  return () => {
    onData.dispose();
    onResize.dispose();
    sendInputRef.current = null;
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
  const [failureReason, setFailureReason] = React.useState<string | null>(null);
  // Bumped by the resume action to force a channel rebind (surfaceResolveOrSpawn resumes a
  // failed/exited surface record) without recreating the Terminal.
  const [resumeKey, setResumeKey] = React.useState(0);

  const termRef = React.useRef<Terminal | null>(null);
  const fitAddonRef = React.useRef<FitAddon | null>(null);
  const sendInputRef = React.useRef<((bytes: number[]) => void) | null>(null);
  const terminalTheme = useLiveTerminalTheme(termRef);
  // Values-only read to seed the Terminal constructor with persisted typography (the live
  // application stays owned by useTerminalPaneExtras' own hook). The dedicated ref is never
  // assigned a Terminal, so this call's effect no-ops -- it only supplies the initial values.
  const seedTypoRef = React.useRef<Terminal | null>(null);
  const initialTypography = useLiveTerminalTypography(seedTypoRef);
  const initialTypographyRef = React.useRef(initialTypography);
  initialTypographyRef.current = initialTypography;
  const [terminalReady, setTerminalReady] = React.useState(false);

  const detach = useMutation(command("surfaceDetach"));
  const detachRef = React.useRef(detach.mutateAsync);
  detachRef.current = detach.mutateAsync;

  const getSurfaceId = React.useCallback(() => surfaceIdRef.current, []);
  const writeInput = React.useCallback((text: string) => {
    sendInputRef.current?.(Array.from(new TextEncoder().encode(text)));
  }, []);
  const extras = useTerminalPaneExtras({
    sessionId: _props.sessionId,
    getSurfaceId,
    writeInput,
    containerRef,
  });
  const attach = extras.attach;

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
        fitAddonRef,
        setTerminalReady,
        detachOnUnmount,
        surfaceIdRef,
        (id) => void detachRef.current({ id }),
        initialTypographyRef.current,
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

  // Attach the pane extras (search/links/bell/copy-paste/drop) once the Terminal exists.
  React.useEffect(() => {
    if (!terminalReady || !termRef.current || !fitAddonRef.current) return () => {};
    return bridgeSubscribe(attach(termRef.current, fitAddonRef.current));
  }, [terminalReady, attach]);

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
        sendInputRef,
        setFailureReason,
      ),
    );
    return () => {
      abort.cancelled = true;
      unsub();
    };
  }, [terminalReady, _props.sessionId, _props.placement, _props.reloadKey, resumeKey]);

  const handleResume = React.useCallback(() => {
    setFailureReason(null);
    setResumeKey((k) => k + 1);
  }, []);

  const handleDismiss = React.useCallback(() => {
    setFailureReason(null);
  }, []);

  const dotColorClass =
    status === "connected"
      ? "bg-terminal-success"
      : status === "exited"
        ? "bg-terminal-error"
        : "bg-terminal-muted";

  return (
    <EntityContextMenu
      entityId={surfaceId ?? _props.placement}
      entityKind="terminal"
      guards={{ "menu.hasSelection": extras.hasSelection }}
      className="h-full w-full relative block"
      style={{ background: terminalTheme.background }}
      data-surface-id={surfaceId ?? undefined}
      onPointerDownCapture={extras.onPointerDownCapture}
    >
      <div
        ref={containerRef}
        className="h-full w-full"
        style={{ padding: "0.333rem 0.333rem 0" }}
      />
      {extras.overlay}
      <div className="absolute top-2 right-3 flex items-center gap-1.5 pointer-events-none">
        <span className={`w-2 h-2 rounded-full inline-block ${dotColorClass}`} />
        <span className="text-terminal-muted text-[0.917rem]">{status}</span>
      </div>
      {failureReason && (
        <TerminalFailureOverlay
          reason={failureReason}
          onResume={handleResume}
          onDismiss={handleDismiss}
        />
      )}
    </EntityContextMenu>
  );
}
