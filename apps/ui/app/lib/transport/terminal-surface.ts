import type { TerminalSurfaceTransport } from "@tillerd/sdk/orchestrator";

type TauriChannelCtor = new <T>() => {
  onmessage: (response: T) => void;
};

// Await once before use: it preloads the Channel ctor so createByteChannel stays sync.
export async function loadTerminalSurfaceTransport(): Promise<TerminalSurfaceTransport> {
  const { invoke, Channel } = await import("@tauri-apps/api/core");
  const ChannelCtor = Channel as unknown as TauriChannelCtor;

  return {
    invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
      return invoke<T>(command, args);
    },

    async listen<T>(event: string, handler: (payload: T) => void): Promise<() => void> {
      const { listen } = await import("@tauri-apps/api/event");
      return listen<T>(event, (e) => handler(e.payload));
    },

    createByteChannel(onBytes: (bytes: Uint8Array) => void): unknown {
      const channel = new ChannelCtor<unknown>();
      channel.onmessage = (msg) => {
        const bytes = toUint8Array(msg);
        if (bytes) onBytes(bytes);
      };
      return channel;
    },
  };
}

function toUint8Array(data: unknown): Uint8Array | null {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (Array.isArray(data)) return new Uint8Array(data as number[]);
  return null;
}
