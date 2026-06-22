import { FramedDaemonTransport } from "./framed";

/** Minimal shape of a Tauri v2 `Channel` (inbound carrier) -- only what the transport touches. */
export interface TauriChannelLike {
  onmessage: ((data: unknown) => void) | null;
}

/** Injected Tauri core surface; the default binds to `@tauri-apps/api/core` at construction. */
export interface TauriCore {
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  createChannel(): TauriChannelLike;
}

export const DAEMON_CONNECT = "daemon_connect";
export const DAEMON_SEND = "daemon_send";
export const DAEMON_DISCONNECT = "daemon_disconnect";

/**
 * Native (web-view) `DaemonTransport`: inbound daemon bytes arrive over a Tauri Channel, outbound
 * frames go over `invoke` -- raw bytes only, the Rust core never parses a frame. Ordering across
 * the Channel hop preserves the daemon flow-control credit/ack loop (ADR-0007): ack frames the
 * renderer emits as it drains travel back over {@link DAEMON_SEND} verbatim.
 */
export class TauriDaemonTransport extends FramedDaemonTransport {
  constructor(private readonly core: TauriCore) {
    super();
  }

  protected openCarrier(): void {
    const channel = this.core.createChannel();
    channel.onmessage = (data) => {
      const bytes = toBytes(data);
      if (bytes) this.onCarrierBytes(bytes);
    };
    this.core
      .invoke(DAEMON_CONNECT, { channel })
      .then(() => this.onCarrierOpen())
      .catch(() => this.onCarrierClose());
  }

  protected writeBytes(bytes: Uint8Array): void {
    // Outbound is low-volume (keystrokes, flow-control acks); JSON number[] is fine. The
    // high-volume daemon -> renderer path rides the raw Channel in openCarrier().
    void this.core.invoke(DAEMON_SEND, { bytes: Array.from(bytes) });
  }

  protected closeCarrier(): void {
    void this.core.invoke(DAEMON_DISCONNECT);
  }
}

function toBytes(data: unknown): Uint8Array | null {
  if (data instanceof Uint8Array) return data;
  if (data instanceof ArrayBuffer) return new Uint8Array(data);
  if (Array.isArray(data)) return new Uint8Array(data as number[]);
  return null;
}
