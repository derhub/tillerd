import {
  FrameDecoder,
  encodeFrame,
  parseDaemonFrame,
  AtError,
  type DaemonFrame,
  type DaemonTransport,
  type FrameHandler,
} from "@athing/sdk";

export class DaemonClient implements DaemonTransport {
  private socket: ReturnType<typeof Bun.connect> | null = null;
  private decoder = new FrameDecoder();
  private sessionHandlers = new Map<string, Set<FrameHandler>>();
  private globalHandlers = new Set<FrameHandler>();
  private pendingList: Array<(ids: string[]) => void> = [];
  private closeHandlers = new Set<() => void>();

  constructor(private readonly socketPath: string) {}

  async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      let handshakeDone = false;

      Bun.connect({
        unix: this.socketPath,
        socket: {
          open: (socket) => {
            this.socket = socket as unknown as ReturnType<typeof Bun.connect>;
            socket.write(encodeFrame({ type: "hello", versions: [1], capabilities: ["snapshot"] }));
          },
          data: (_socket, chunk) => {
            const raw =
              typeof chunk === "string"
                ? Buffer.from(chunk, "utf8")
                : Buffer.from(chunk as unknown as ArrayBuffer);

            for (const { meta, body } of this.decoder.push(raw)) {
              if (!handshakeDone) {
                const frame = parseDaemonFrame(meta);
                if (frame?.type === "hello-ack") {
                  handshakeDone = true;
                  resolve();
                } else if (frame?.type === "error") {
                  reject(new AtError("TransportClosed", frame.message));
                } else {
                  reject(new AtError("TransportClosed", "unexpected frame before hello-ack"));
                }
                continue;
              }
              const frame = parseDaemonFrame(meta);
              if (frame) this.dispatch(frame, body);
            }
          },
          close: () => {
            this.socket = null;
            // Drain any pending list() callers so they don't hang forever.
            for (const cb of this.pendingList.splice(0)) cb([]);
            for (const h of this.closeHandlers) h();
          },
          error: (_socket, err) => {
            if (!handshakeDone) reject(err);
          },
        },
      });
    });
  }

  private dispatch(frame: DaemonFrame, body: Uint8Array | null): void {
    if (frame.type === "list-ack") {
      for (const cb of this.pendingList.splice(0)) cb(frame.ids);
      return;
    }

    const sid = "sessionId" in frame ? frame.sessionId : undefined;
    if (sid) {
      const handlers = this.sessionHandlers.get(sid);
      if (handlers) {
        for (const h of handlers) h(frame, body);
      }
    }
    for (const h of this.globalHandlers) h(frame, body);
  }

  send(meta: object, body?: Uint8Array): void {
    if (!this.socket) throw new AtError("TransportClosed", "daemon not connected");
    (this.socket as unknown as { write(data: Uint8Array): void }).write(encodeFrame(meta, body));
  }

  subscribe(sessionId: string, handler: FrameHandler): () => void {
    if (!this.sessionHandlers.has(sessionId)) {
      this.sessionHandlers.set(sessionId, new Set());
    }
    this.sessionHandlers.get(sessionId)!.add(handler);
    return () => this.sessionHandlers.get(sessionId)?.delete(handler);
  }

  async list(): Promise<string[]> {
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
    if (this.socket) {
      (this.socket as unknown as { end(): void }).end();
      this.socket = null;
    }
  }
}
