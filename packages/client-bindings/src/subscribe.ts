import type { NotificationWire, StatusWire } from "./tauri_bindings.gen";

import { openChannel } from "./channel";
import { ensureResult } from "./readiness";
import { commands } from "./tauri_bindings.gen";

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
  const handle = await openChannel<
    { kind: "input"; bytes: number[] } | { kind: "resize"; cols: number; rows: number }
  >(
    (channel) =>
      commands.surfaceChannel({ channel, req: { surfaceId: params.surfaceId } }).then(ensureResult),
    (bytes) => {
      callback(decodeSurfaceEvent(bytes));
    },
    {
      send: async (msg) => {
        await commands.surfaceChannelSend({ key: params.surfaceId, msg }).then(ensureResult);
      },
      close: async () => {
        await commands
          .surfaceChannelClose({ req: { surfaceId: params.surfaceId } })
          .then(ensureResult);
      },
    },
  );

  return {
    send: (msg) => handle.send!(msg),
    close: () => handle.close(),
  };
}

export type LogChannelHandle = {
  close(): Promise<void>;
};

export async function logChannel(
  params: { service: string },
  callback: (bytes: Uint8Array) => void,
): Promise<LogChannelHandle> {
  return openChannel(
    (channel) =>
      commands.logChannel({ channel, req: { service: params.service } }).then(ensureResult),
    (bytes) => {
      if (bytes[0] === 0x00) {
        callback(bytes.subarray(1));
      }
    },
    {
      close: async () => {
        await commands.logChannelClose({ req: { service: params.service } }).then(ensureResult);
      },
    },
  );
}

export type NotificationChannelHandle = {
  close(): Promise<void>;
};

export async function notificationChannel(
  callback: (event: NotificationWire) => void,
): Promise<NotificationChannelHandle> {
  const channelId = crypto.randomUUID();
  return openChannel(
    (channel) => commands.notificationChannel({ channel, req: { channelId } }).then(ensureResult),
    (bytes) => {
      if (bytes[0] === 0x00) {
        const json = new TextDecoder().decode(bytes.subarray(1));
        callback(JSON.parse(json));
      }
    },
    {
      close: async () => {
        await commands.notificationChannelClose({ req: { channelId } }).then(ensureResult);
      },
    },
  );
}

export type SurfaceStatusEvent = {
  surfaceId: string;
  sessionId: string;
  workspaceId: string;
  status: string;
};

export type SurfaceStatusChannelHandle = {
  close(): Promise<void>;
};

// Per-window subscription to surface runtime status transitions (ADR-0044): the
// orchestrator pushes after each status write commits, so a re-query on receipt
// always reads the post-transition row.
export async function surfaceStatusChannel(
  callback: (event: SurfaceStatusEvent) => void,
): Promise<SurfaceStatusChannelHandle> {
  const channelId = crypto.randomUUID();
  return openChannel(
    (channel) => commands.surfaceStatusChannel({ channel, req: { channelId } }).then(ensureResult),
    (bytes) => {
      if (bytes[0] === 0x00) {
        const json = new TextDecoder().decode(bytes.subarray(1));
        callback(JSON.parse(json));
      }
    },
    {
      close: async () => {
        await commands.surfaceStatusChannelClose({ req: { channelId } }).then(ensureResult);
      },
    },
  );
}

export type LogsChangedChannelHandle = {
  close(): Promise<void>;
};

export async function logsChangedChannel(callback: () => void): Promise<LogsChangedChannelHandle> {
  const channelId = crypto.randomUUID();
  return openChannel(
    (channel) => commands.logsChangedChannel({ channel, req: { channelId } }).then(ensureResult),
    () => {
      callback();
    },
    {
      close: async () => {
        await commands.logsChangedChannelClose({ req: { channelId } }).then(ensureResult);
      },
    },
  );
}
