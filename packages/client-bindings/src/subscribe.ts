import type { EventCallback } from "@tauri-apps/api/event";
import type { StatusWire } from "./tauri_bindings.gen";

import { Channel } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";

import { commands } from "./tauri_bindings.gen";
import { ensureResult } from "./readiness";

type TauriEvent<T> = { listen: (cb: EventCallback<T>) => Promise<() => void> };

// Subscribe to a Tauri event in a React component. Cleans up on unmount. The callback is held in a
// ref so it stays current across renders without re-subscribing (no useCallback required at call sites).
export function useEventSub<T>(evt: TauriEvent<T>, cb: EventCallback<T>): void {
  const ref = useRef(cb);
  ref.current = cb;
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void evt
      .listen((e) => ref.current(e))
      .then((u) => {
        unlisten = u;
      });
    return () => {
      unlisten?.();
    };
  }, [evt]);
}

// The orchestrator status is the readiness-bootstrap command: it runs BEFORE setReady, so it stays
// outside the readiness-gated query()/command() wrappers and returns StatusWire directly (not a Result).
export function orchestratorStatus(): Promise<StatusWire> {
  return commands.orchestratorStatus();
}

export type StreamHandle<T> = {
  channel: Channel<T>;
  teardown: () => void;
};

/** Create a typed Channel for any keyed stream; wire `onmessage` and return a teardown handle. */
export function makeStreamChannel<T>(onmessage: (frame: T) => void): StreamHandle<T> {
  const channel = new Channel<T>();
  channel.onmessage = onmessage;
  return { channel, teardown: () => { channel.onmessage = () => undefined; } };
}

/** Create a typed Channel for PTY byte streams (pass to commands.surfaceChannel). */
export function makeSurfaceChannel(): Channel<number[]> {
  return new Channel<number[]>();
}

export type ChannelHandle = {
  readonly surfaceId: string;
  set onmessage(fn: (frame: number[]) => void);
  send(bytes: Uint8Array): Promise<void>;
  resize(cols: number, rows: number): Promise<void>;
  close(): Promise<void>;
};

/** Teardown for a live log subscription: detaches the channel and unsubscribes the service. */
export type LogStreamHandle = {
  teardown: () => Promise<void>;
};

/**
 * Subscribe to the live log stream for one service (the log file-name prefix, e.g.
 * `tillerd-daemon`). `onLine` is called for every appended line. Returns a handle whose
 * `teardown` stops delivery and unsubscribes the service.
 */
export async function subscribeLogs(
  service: string,
  onLine: (line: string) => void,
): Promise<LogStreamHandle> {
  const channel = new Channel<string>();
  channel.onmessage = onLine;
  ensureResult(await commands.logSubscribe({ channel, service }));
  return {
    teardown(): Promise<void> {
      channel.onmessage = () => undefined;
      return commands
        .logUnsubscribe({ service })
        .then(ensureResult)
        .then(() => undefined);
    },
  };
}

export type SurfaceChannelParams = {
  sessionId: string;
  placement: string;
  cols: number;
  rows: number;
  cwd?: string | null;
};

export async function openSurfaceChannel(params: SurfaceChannelParams): Promise<ChannelHandle> {
  const channel = new Channel<number[]>();
  const surfaceId = ensureResult(
    await commands.surfaceChannel({
      channel,
      sessionId: params.sessionId,
      placement: params.placement,
      cols: params.cols,
      rows: params.rows,
      cwd: params.cwd ?? null,
    }),
  );
  return {
    surfaceId,
    set onmessage(fn: (frame: number[]) => void) {
      channel.onmessage = fn;
    },
    send(bytes: Uint8Array): Promise<void> {
      return commands
        .surfaceChannelSendCmd({ key: surfaceId, msg: { kind: "input", bytes: Array.from(bytes) } })
        .then(ensureResult)
        .then(() => undefined);
    },
    resize(cols: number, rows: number): Promise<void> {
      return commands
        .surfaceChannelSendCmd({ key: surfaceId, msg: { kind: "resize", cols, rows } })
        .then(ensureResult)
        .then(() => undefined);
    },
    close(): Promise<void> {
      channel.onmessage = () => undefined;
      return commands
        .surfaceChannelSendCmd({ key: surfaceId, msg: { kind: "close" } })
        .then(ensureResult)
        .then(() => undefined);
    },
  };
}
