import type { EventCallback } from "@tauri-apps/api/event";

import { Channel } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";

import type { NotificationWire, StatusWire } from "./tauri_bindings.gen";

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

export type NotificationChannelHandle = {
  close(): Promise<void>;
};

function randomId(): string {
  const c =
    typeof globalThis !== "undefined" ? (globalThis.crypto as Crypto | undefined) : undefined;
  if (c && typeof c.randomUUID === "function") return c.randomUUID();
  if (c && typeof c.getRandomValues === "function") {
    const bytes = c.getRandomValues(new Uint8Array(16));
    bytes[6] = (bytes[6]! & 0x0f) | 0x40;
    bytes[8] = (bytes[8]! & 0x3f) | 0x80;
    const hex = Array.from(bytes, (n) => n.toString(16).padStart(2, "0"));
    return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex
      .slice(6, 8)
      .join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10, 16).join("")}`;
  }
  const hex = Array.from({ length: 16 }, () =>
    Math.floor(Math.random() * 256)
      .toString(16)
      .padStart(2, "0"),
  );
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex
    .slice(6, 8)
    .join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10, 16).join("")}`;
}

export async function notificationChannel(
  callback: (event: NotificationWire) => void,
): Promise<NotificationChannelHandle> {
  const channel = new Channel<number[]>();
  const channelId = randomId();
  channel.onmessage = (data) => {
    const bytes = new Uint8Array(data);
    if (bytes[0] === 0x00) {
      const json = new TextDecoder().decode(bytes.subarray(1));
      callback(JSON.parse(json));
    }
  };

  await commands.notificationChannel({ channel, req: { channelId } }).then(ensureResult);

  return {
    async close() {
      channel.onmessage = () => {};
      await commands.notificationChannelClose({ req: { channelId } }).then(ensureResult);
    },
  };
}
