import { toBytes } from "./bytes";
import { FramedDaemonTransport } from "./framed";

export interface WebSocketLike {
  binaryType: string;
  send(data: ArrayBufferView): void;
  close(): void;
  onopen: ((ev: unknown) => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onclose: ((ev: unknown) => void) | null;
  onerror: ((ev: unknown) => void) | null;
}

export type WebSocketFactory = (url: string) => WebSocketLike;

const defaultFactory: WebSocketFactory = (url) => new WebSocket(url) as unknown as WebSocketLike;

export class WebSocketDaemonTransport extends FramedDaemonTransport {
  private ws: WebSocketLike | null = null;

  constructor(
    private readonly url: string,
    private readonly factory: WebSocketFactory = defaultFactory,
  ) {
    super();
  }

  protected openCarrier(): void {
    const ws = this.factory(this.url);
    ws.binaryType = "arraybuffer";
    this.ws = ws;
    ws.onopen = () => this.onCarrierOpen();
    ws.onmessage = (ev) => {
      const bytes = toBytes(ev.data);
      if (bytes) this.onCarrierBytes(bytes);
    };
    ws.onclose = () => {
      this.ws = null;
      this.onCarrierClose();
    };
    ws.onerror = () => {
      if (!this.open) this.onCarrierClose();
    };
  }

  protected writeBytes(bytes: Uint8Array): void {
    this.ws?.send(bytes);
  }

  protected closeCarrier(): void {
    this.ws?.close();
    this.ws = null;
  }
}
