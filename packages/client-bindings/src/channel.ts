import { Channel } from "@tauri-apps/api/core";

export interface ChannelHandle<TSend = void> {
  close(): Promise<void>;
  send?: (msg: TSend) => Promise<void>;
}

export async function openChannel<TSend = void, TOpenResult = unknown>(
  openFn: (channel: Channel<number[]>) => Promise<TOpenResult>,
  onMessage: (data: Uint8Array) => void,
  options?: {
    send?: (msg: TSend) => Promise<void>;
    close?: () => Promise<void>;
  },
): Promise<ChannelHandle<TSend>> {
  const channel = new Channel<number[]>();
  channel.onmessage = (data) => {
    onMessage(new Uint8Array(data));
  };

  await openFn(channel);

  return {
    send: options?.send,
    close: async () => {
      channel.onmessage = () => {};
      if (options?.close) {
        await options.close();
      }
    },
  };
}
