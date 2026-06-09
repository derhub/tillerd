import {
  FrameDecoder,
  encodeFrame,
  parseDaemonFrame,
  AtError,
  type DaemonFrame,
  type DaemonTransport,
  type FrameHandler,
} from "@tillerd/sdk";

/**
 * Carrier-agnostic `DaemonTransport`. Owns the single-sourced sdk framing codec, the daemon
 * handshake, and frame dispatch; subclasses supply only the byte carrier (WebSocket, Tauri
 * Channel + `invoke`, …). Raw bytes only — no frame is ever parsed in the carrier.
 */
export abstract class FramedDaemonTransport implements DaemonTransport {
  private readonly decoder = new FrameDecoder();
  private readonly sessionHandlers = new Map<string, Set<FrameHandler>>();
  private readonly globalHandlers = new Set<FrameHandler>();
  private readonly pendingList: Array<(ids: string[]) => void> = [];
  private readonly closeHandlers = new Set<() => void>();
  private handshakeDone = false;
  private resolveConnect: (() => void) | null = null;
  private rejectConnect: ((err: unknown) => void) | null = null;
  protected open = false;

  /** Open the carrier; drive {@link onCarrierOpen}/{@link onCarrierBytes}/{@link onCarrierClose}. */
  protected abstract openCarrier(): void;
  /** Write framed bytes to the carrier. */
  protected abstract writeBytes(bytes: Uint8Array): void;
  /** Close the carrier. */
  protected abstract closeCarrier(): void;

  connect(): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      this.handshakeDone = false;
      this.resolveConnect = resolve;
      this.rejectConnect = reject;
      this.openCarrier();
    });
  }

  protected onCarrierOpen(): void {
    this.open = true;
    this.writeBytes(encodeFrame({ type: "hello", versions: [1], capabilities: ["snapshot"] }));
  }

  protected onCarrierBytes(bytes: Uint8Array): void {
    for (const { meta, body } of this.decoder.push(bytes)) {
      if (!this.handshakeDone) {
        const frame = parseDaemonFrame(meta);
        if (frame?.type === "hello-ack") {
          this.handshakeDone = true;
          this.resolveConnect?.();
        } else if (frame?.type === "error") {
          this.rejectConnect?.(new AtError("TransportClosed", frame.message));
        } else {
          this.rejectConnect?.(new AtError("TransportClosed", "unexpected frame before hello-ack"));
        }
        continue;
      }
      const frame = parseDaemonFrame(meta);
      if (frame) this.dispatch(frame, body);
    }
  }

  protected onCarrierClose(): void {
    this.open = false;
    if (!this.handshakeDone) {
      this.rejectConnect?.(new AtError("TransportClosed", "carrier closed before hello-ack"));
    }
    for (const cb of this.pendingList.splice(0)) cb([]);
    for (const h of this.closeHandlers) h();
  }

  private dispatch(frame: DaemonFrame, body: Uint8Array | null): void {
    if (frame.type === "list-ack") {
      for (const cb of this.pendingList.splice(0)) cb(frame.ids);
      return;
    }
    const sid = "sessionId" in frame ? frame.sessionId : undefined;
    if (sid) {
      const handlers = this.sessionHandlers.get(sid);
      if (handlers) for (const h of handlers) h(frame, body);
    }
    for (const h of this.globalHandlers) h(frame, body);
  }

  send(meta: object, body?: Uint8Array): void {
    if (!this.open) throw new AtError("TransportClosed", "daemon not connected");
    this.writeBytes(encodeFrame(meta, body));
  }

  subscribe(sessionId: string, handler: FrameHandler): () => void {
    if (!this.sessionHandlers.has(sessionId)) this.sessionHandlers.set(sessionId, new Set());
    this.sessionHandlers.get(sessionId)!.add(handler);
    return () => this.sessionHandlers.get(sessionId)?.delete(handler);
  }

  list(): Promise<string[]> {
    return new Promise((resolve) => {
      this.pendingList.push(resolve);
      this.send({ type: "list" });
    });
  }

  onClose(handler: () => void): () => void {
    this.closeHandlers.add(handler);
    return () => this.closeHandlers.delete(handler);
  }

  disconnect(): void {
    if (this.open) {
      this.open = false;
      this.closeCarrier();
    }
  }
}
