import type { EventCallback } from "@tauri-apps/api/event";

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
