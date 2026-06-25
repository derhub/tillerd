import type { EventCallback } from "@tauri-apps/api/event";

import { Channel } from "@tauri-apps/api/core";
import { useEffect, useRef } from "react";

import type { StatusWire } from "./tauri_bindings.gen";

import { ensureResult } from "./readiness";
import { commands } from "./tauri_bindings.gen";

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

export type StreamHandle = {
  teardown: () => Promise<void>;
};

export type GenericChannelHandle<TSend> = {
  readonly key: string;
  send(msg: TSend): Promise<void>;
  close(): Promise<void>;
};

type Commands = typeof commands;
type CommandKey = keyof Commands;

type Args<K extends CommandKey> =
  Parameters<Commands[K]> extends [infer A, ...unknown[]] ? A : void;

// Distributive (naked R): maps the {ok}|{error} union member-wise.
type OkData<R> = R extends { status: "ok"; data: infer T } ? T : never;
type Result<K extends CommandKey> = OkData<Awaited<ReturnType<Commands[K]>>>;

// 1. Unidirectional Streams (take a Channel)
export type StreamKey = {
  [K in CommandKey]: Args<K> extends { channel: Channel<any> } ? K : never;
}[CommandKey];

export type StreamArgs<K extends StreamKey> = Omit<Args<K>, "channel">;
export type StreamMsg<K extends StreamKey> = 
  Args<K> extends { channel: Channel<infer T> } ? T : never;

// 2. Bidirectional Duplex Channels (take a Channel and return a string session key)
export type ChannelKey = {
  [K in CommandKey]: Args<K> extends { channel: Channel<any> }
    ? Result<K> extends string
      ? K
      : never
    : never;
}[CommandKey];

export type ChannelArgs<K extends ChannelKey> = Omit<Args<K>, "channel">;
export type ChannelRecv<K extends ChannelKey> = 
  Args<K> extends { channel: Channel<infer T> } ? T : never;

// Derive the send command key by convention: [name]Send
type SendCmdKey<K extends string> = `${K}Send` extends CommandKey ? `${K}Send` : never;

export type ChannelSend<K extends ChannelKey> = 
  SendCmdKey<K> extends CommandKey
    ? Args<SendCmdKey<K>> extends { msg: infer M } ? M : never
    : never;

/** Open a generic, type-safe unidirectional stream. */
export async function openStream<K extends StreamKey>(
  cmd: K,
  args: StreamArgs<K>,
  onMessage: (msg: StreamMsg<K>) => void,
): Promise<StreamHandle> {
  const channel = new Channel<StreamMsg<K>>();
  channel.onmessage = onMessage;

  const fullArgs = { ...args, channel } as unknown as Args<K>;
  const run = commands[cmd] as (
    a?: unknown,
  ) => Promise<{ status: "ok"; data: unknown } | { status: "error"; error: unknown }>;
  await run(fullArgs).then(ensureResult);

  return {
    async teardown() {
      channel.onmessage = () => undefined;
      const unsubCmd = (cmd as string).replace("Subscribe", "Unsubscribe") as CommandKey;
      const runUnsub = commands[unsubCmd] as (
        a?: unknown,
      ) => Promise<{ status: "ok"; data: unknown } | { status: "error"; error: unknown }>;
      await runUnsub(args).then(ensureResult);
    },
  };
}

/** Open a generic, type-safe bidirectional duplex channel. */
export async function openChannel<K extends ChannelKey>(
  cmd: K,
  args: ChannelArgs<K>,
  onMessage: (msg: ChannelRecv<K>) => void,
): Promise<GenericChannelHandle<ChannelSend<K>>> {
  const channel = new Channel<ChannelRecv<K>>();
  channel.onmessage = onMessage;

  const fullArgs = { ...args, channel } as unknown as Args<K>;
  const run = commands[cmd] as (
    a?: unknown,
  ) => Promise<{ status: "ok"; data: string } | { status: "error"; error: unknown }>;
  const key = await run(fullArgs).then(ensureResult) as string;

  return {
    key,
    send(msg: ChannelSend<K>) {
      const sendCmd = `${cmd as string}Send` as CommandKey;
      const runSend = commands[sendCmd] as (
        a?: unknown,
      ) => Promise<{ status: "ok"; data: unknown } | { status: "error"; error: unknown }>;
      return runSend({ key, msg }).then(ensureResult).then(() => undefined);
    },
    async close() {
      channel.onmessage = () => undefined;
      const sendCmd = `${cmd as string}Send` as CommandKey;
      const runSend = commands[sendCmd] as (
        a?: unknown,
      ) => Promise<{ status: "ok"; data: unknown } | { status: "error"; error: unknown }>;
      await runSend({ key, msg: { kind: "close" } }).then(ensureResult);
    },
  };
}

export type ChannelHandle = {
  readonly surfaceId: string;
  set onmessage(fn: (frame: number[]) => void);
  send(bytes: Uint8Array): Promise<void>;
  resize(cols: number, rows: number): Promise<void>;
  close(): Promise<void>;
};

export type LogStreamHandle = StreamHandle;

/**
 * Subscribe to the live log stream for one service (the log file-name prefix, e.g.
 * `tillerd-daemon`). `onLine` is called for every appended line. Returns a handle whose
 * `teardown` stops delivery and unsubscribes the service.
 * @deprecated Use openStream('logSubscribe', { service }, onLine) instead.
 */
export function subscribeLogs(
  service: string,
  onLine: (line: string) => void,
): Promise<LogStreamHandle> {
  return openStream("logSubscribe", { service }, onLine);
}

export type SurfaceChannelParams = {
  sessionId: string;
  placement: string;
  cols: number;
  rows: number;
  cwd?: string | null;
};

/**
 * Open (or revisit) a surface duplex channel.
 * @deprecated Use openChannel('surfaceChannel', { sessionId, placement, cols, rows, cwd }, onmessage) instead.
 */
export async function openSurfaceChannel(params: SurfaceChannelParams): Promise<ChannelHandle> {
  let onMsgCallback: ((frame: number[]) => void) | undefined;

  const innerHandle = await openChannel(
    "surfaceChannel",
    {
      sessionId: params.sessionId,
      placement: params.placement,
      cols: params.cols,
      rows: params.rows,
      cwd: params.cwd ?? null,
    },
    (frame) => {
      onMsgCallback?.(frame);
    },
  );

  return {
    get surfaceId() {
      return innerHandle.key;
    },
    set onmessage(fn: (frame: number[]) => void) {
      onMsgCallback = fn;
    },
    send(bytes: Uint8Array): Promise<void> {
      return innerHandle.send({ kind: "input", bytes: Array.from(bytes) } as any);
    },
    resize(cols: number, rows: number): Promise<void> {
      return innerHandle.send({ kind: "resize", cols, rows } as any);
    },
    close(): Promise<void> {
      return innerHandle.close();
    },
  };
}
