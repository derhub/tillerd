import type { SurfaceChannelHandle } from "@tillerd/client-bindings";
import type { FitAddon } from "@xterm/addon-fit";
import type { IDisposable, Terminal } from "@xterm/xterm";

import { useMutation } from "@tanstack/react-query";
import { command, runCommand, surfaceChannel } from "@tillerd/client-bindings";
import "@xterm/xterm/css/xterm.css";
import React from "react";

import { EntityContextMenu } from "~/components/shell/EntityContextMenu";
import { TerminalExitBar } from "~/components/terminal/TerminalExitBar";
import { TerminalFailureOverlay } from "~/components/terminal/TerminalFailureOverlay";
import { usePaneShortcutDispatch } from "~/lib/commands/usePaneShortcuts";
import { lazyFitAddon, lazyXterm } from "~/lib/lazy";
import { getTerminalTheme } from "~/lib/settings/terminal-schemes";
import { useLiveTerminalTheme } from "~/lib/settings/useLiveTerminalTheme";
import {
  useTerminalTypography,
  type TerminalTypography,
} from "~/lib/settings/useLiveTerminalTypography";
import { subscribe as bridgeSubscribe } from "~/lib/subscribe";
import { isCleanExit } from "~/lib/terminal/exit-classification";

import { useTerminalPaneExtras } from "./useTerminalPaneExtras";

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

  // Seed construction with the persisted typography so the first paint uses it instead of the
  // defaults, avoiding an initial reflow/refit flash; useTerminalPaneExtras re-applies the live
  // values on attach and on every subsequent change.
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
  setSendInput: (fn: ((bytes: number[]) => void) | null) => void,
  setFailureReason: (reason: string | null) => void,
  setExitQualifier: (q: string | null) => void,
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
        if (isCleanExit(event.value)) {
          // Clean exit: hold the scrollback and let the pane show the restart bar (surface-lifecycle
          // spec). The qualifier drives the bar's label.
          setExitQualifier(event.value);
        } else {
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
  setExitQualifier(null);

  const encoder = new TextEncoder();
  const onData: IDisposable = term.onData((data) => {
    void handle.send({ kind: "input", bytes: Array.from(encoder.encode(data)) });
  });
  const onResize: IDisposable = term.onResize(({ cols, rows }) => {
    void handle.send({ kind: "resize", cols, rows });
  });
  // Publish the input path so the extras layer (path drop) can write to this surface's PTY.
  setSendInput((bytes) => void handle.send({ kind: "input", bytes }));

  return () => {
    onData.dispose();
    onResize.dispose();
    setSendInput(null);
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
  // Lifecycle callbacks (surface-lifecycle spec). onStatusChange lifts the pane's connection status
  // to the tree owner (backs the confirm-if-running close gate); onRequestReset asks the owner to
  // unbind this leaf to the empty picker (exit bar "New surface", failure Dismiss). Restart is
  // handled in-pane (resolveOrSpawn resumes the same placement), so no owner callback is needed.
  onStatusChange?: (placement: string, status: string) => void;
  onRequestReset?: () => void;
}) {
  const detachOnUnmount = _props.detachOnUnmount ?? true;
  const containerRef = React.useRef<HTMLDivElement>(null);
  const [status, setStatus] = React.useState<string>("connecting");
  const [surfaceId, setSurfaceId] = React.useState<string | null>(null);
  const surfaceIdRef = React.useRef<string | null>(null);
  surfaceIdRef.current = surfaceId;
  const [failureReason, setFailureReason] = React.useState<string | null>(null);
  const [exitQualifier, setExitQualifier] = React.useState<string | null>(null);

  // Lift status to the tree owner so its close-confirm gate knows whether a live process still runs.
  const onStatusChange = _props.onStatusChange;
  const placement = _props.placement;
  React.useEffect(() => {
    onStatusChange?.(placement, status);
  }, [onStatusChange, placement, status]);
  // Bumped by the resume action to force a channel rebind (surfaceResolveOrSpawn resumes a
  // failed/exited surface record) without recreating the Terminal.
  const [resumeKey, setResumeKey] = React.useState(0);

  const termRef = React.useRef<Terminal | null>(null);
  const fitAddonRef = React.useRef<FitAddon | null>(null);
  const sendInputRef = React.useRef<((bytes: number[]) => void) | null>(null);
  const terminalTheme = useLiveTerminalTheme(termRef);
  // Seed the Terminal constructor with persisted typography for the first paint; the live
  // application stays owned by useTerminalPaneExtras' own useLiveTerminalTypography.
  const initialTypography = useTerminalTypography();
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
  const dispatchPaneKey = usePaneShortcutDispatch();
  const extras = useTerminalPaneExtras({
    sessionId: _props.sessionId,
    getSurfaceId,
    writeInput,
    containerRef,
    onPaneKey: dispatchPaneKey,
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

    const guard = <T extends any[]>(fn: (...args: T) => void) => {
      return (...args: T) => {
        if (!abort.cancelled) fn(...args);
      };
    };

    const unsub = bridgeSubscribe(
      bindChannel(
        abort,
        termRef.current,
        _props.sessionId,
        _props.placement,
        guard(setSurfaceId),
        guard(setStatus),
        (fn) => {
          if (!abort.cancelled) sendInputRef.current = fn;
        },
        guard(setFailureReason),
        guard(setExitQualifier),
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

  // Restart an exited pane in place (surface-lifecycle spec): resolveOrSpawn resumes the same
  // (session, placement) -- for an idle/cleanly-exited surface it spawns a fresh PTY while keeping
  // the surface id and placement (verified: orchestrator resolve_or_spawn resume branch). So restart
  // is a local rebind: clear the dead scrollback and bump resumeKey to re-run bindChannel.
  const handleRestart = React.useCallback(() => {
    termRef.current?.reset();
    setExitQualifier(null);
    setFailureReason(null);
    setStatus("connecting");
    setResumeKey((k) => k + 1);
  }, []);

  // Dismiss on failure resets the leaf to the empty picker (surface-lifecycle spec) rather than
  // leaving a dead pane; the owner terminates the failed surface and unbinds the leaf.
  const onRequestReset = _props.onRequestReset;
  const handleDismiss = React.useCallback(() => {
    setFailureReason(null);
    onRequestReset?.();
  }, [onRequestReset]);

  const handleReconnect = React.useCallback(() => {
    termRef.current?.clear();
    setFailureReason(null);
    setExitQualifier(null);
    setStatus("connecting");
    setResumeKey((k) => k + 1);
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
      <button
        type="button"
        onClick={handleReconnect}
        aria-label="Reconnect terminal"
        className="absolute top-2 right-3 flex items-center gap-1.5 px-1.5 py-0.5 rounded text-[0.833rem] text-terminal-muted hover:text-foreground hover:bg-terminal-surface/50 border border-transparent hover:border-terminal-border transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring select-none cursor-pointer"
        data-testid="terminal-status-reconnect"
      >
        <span className={`w-2 h-2 rounded-full inline-block ${dotColorClass}`} />
        <span>{status}</span>
      </button>
      {failureReason && (
        <TerminalFailureOverlay
          reason={failureReason}
          onResume={handleResume}
          onDismiss={handleDismiss}
        />
      )}
      {!failureReason && exitQualifier !== null && (
        <TerminalExitBar
          qualifier={exitQualifier}
          onRestart={handleRestart}
          onNewSurface={() => _props.onRequestReset?.()}
        />
      )}
    </EntityContextMenu>
  );
}
