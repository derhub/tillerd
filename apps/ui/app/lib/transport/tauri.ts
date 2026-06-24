import { toBytes } from "./bytes";
import { FramedDaemonTransport } from "./framed";

export interface TauriChannelLike {
  onmessage: ((data: unknown) => void) | null;
}

export interface TauriCore {
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown>): Promise<T>;
  createChannel(): TauriChannelLike;
  listen<T = unknown>(event: string, handler: (payload: T) => void): Promise<() => void>;
}

export const DAEMON_CONNECT = "daemon_connect";
export const DAEMON_SEND = "daemon_send";
export const DAEMON_DISCONNECT = "daemon_disconnect";

// Raw bytes only -- Rust core never parses a frame. Channel ordering preserves the flow-control credit/ack loop.
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
    // Outbound is low-volume (keystrokes, acks); JSON number[] is acceptable. High-volume inbound rides the raw Channel in openCarrier().
    void this.core.invoke(DAEMON_SEND, { bytes: Array.from(bytes) });
  }

  protected closeCarrier(): void {
    void this.core.invoke(DAEMON_DISCONNECT);
  }
}
