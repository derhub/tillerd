import type { EventCallback } from "@tauri-apps/api/event";

import { Channel } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";

import type { StatusWire } from "./tauri_bindings.gen";

import { ensureResult } from "./readiness";
import { commands } from "./tauri_bindings.gen";

type TauriEvent<T> = { listen: (cb: EventCallback<T>) => Promise<() => void> };

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

export function orchestratorStatus(): Promise<StatusWire> {
  return commands.orchestratorStatus();
}

export type SurfaceChannelEvent =
  | { kind: "bytes"; value: Uint8Array }
  | { kind: "status"; value: string }
  | { kind: "exit"; value: string }
  | { kind: "error"; value: string };

function decodeSurfaceEvent(data: Uint8Array): SurfaceChannelEvent {
  const type = data[0];
  const payload = data.subarray(1);
  const textDecoder = new TextDecoder();
  switch (type) {
    case 0x00:
      return { kind: "bytes", value: payload };
    case 0x01:
      return { kind: "status", value: textDecoder.decode(payload) };
    case 0x02:
      return { kind: "exit", value: textDecoder.decode(payload) };
    case 0x03:
      return { kind: "error", value: textDecoder.decode(payload) };
    default:
      return { kind: "error", value: `unknown event type: ${type}` };
  }
}

export type SurfaceChannelHandle = {
  send(
    msg: { kind: "input"; bytes: number[] } | { kind: "resize"; cols: number; rows: number },
  ): Promise<void>;
  close(): Promise<void>;
};

export async function surfaceChannel(
  params: { surfaceId: string },
  callback: (event: SurfaceChannelEvent) => void,
): Promise<SurfaceChannelHandle> {
  const channel = new Channel<number[]>();
  channel.onmessage = (data) => {
    callback(decodeSurfaceEvent(new Uint8Array(data)));
  };

  await commands
    .surfaceChannel({ channel, req: { surfaceId: params.surfaceId } })
    .then(ensureResult);

  return {
    async send(msg) {
      await commands.surfaceChannelSend({ key: params.surfaceId, msg }).then(ensureResult);
    },
    async close() {
      channel.onmessage = () => {};
      await commands
        .surfaceChannelClose({ req: { surfaceId: params.surfaceId } })
        .then(ensureResult);
    },
  };
}

export type LogChannelHandle = {
  close(): Promise<void>;
};

export async function logChannel(
  params: { service: string },
  callback: (bytes: Uint8Array) => void,
): Promise<LogChannelHandle> {
  const channel = new Channel<number[]>();
  channel.onmessage = (data) => {
    const bytes = new Uint8Array(data);
    if (bytes[0] === 0x00) {
      callback(bytes.subarray(1));
    }
  };

  await commands.logChannel({ channel, req: { service: params.service } }).then(ensureResult);

  return {
    async close() {
      channel.onmessage = () => {};
      await commands.logChannelClose({ req: { service: params.service } }).then(ensureResult);
    },
  };
}
