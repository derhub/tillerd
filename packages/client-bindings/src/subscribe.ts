import type { EventCallback } from "@tauri-apps/api/event";

import { Channel } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";

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

/** Create a typed Channel for PTY byte streams (pass to commands.surfaceCreate). */
export function makeSurfaceChannel(): Channel<number[]> {
  return new Channel<number[]>();
}
