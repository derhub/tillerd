// Minimal benchmark client: speaks the real daemon control-socket protocol so
// measured cost reflects the full path (framing, fan-out, snapshot production),
// not internal function calls.

import { encodeFrame, FrameDecoder } from "@athing/sdk/protocol";

export type Frame = { meta: any; body: Uint8Array | null };
type Handler = (f: Frame) => void;

export class BenchClient {
  private socket: any = null;
  private decoder = new FrameDecoder();
  private handlers = new Set<Handler>();
  private pendingList: Array<(ids: string[]) => void> = [];

  constructor(
    private readonly socketPath: string,
    private readonly capabilities: string[] = ["snapshot"],
  ) {}

  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      let handshakeDone = false;
      Bun.connect({
        unix: this.socketPath,
        socket: {
          open: (socket) => {
            this.socket = socket;
            socket.write(
              encodeFrame({ type: "hello", versions: [1], capabilities: this.capabilities }),
            );
          },
          data: (_s, chunk) => {
            const raw = Buffer.from(chunk as any);
            for (const { meta, body } of this.decoder.push(raw)) {
              const m = meta as any;
              if (!handshakeDone) {
                if (m?.type === "hello-ack") {
                  handshakeDone = true;
                  resolve();
                } else {
                  reject(new Error("unexpected pre-handshake frame: " + JSON.stringify(m)));
                }
                continue;
              }
              if (m?.type === "list-ack") {
                for (const cb of this.pendingList.splice(0)) cb(m.ids);
              }
              for (const h of this.handlers) h({ meta: m, body });
            }
          },
          error: (_s, e) => {
            if (!handshakeDone) reject(e);
          },
          close: () => {
            this.socket = null;
          },
        },
      });
    });
  }

  on(h: Handler): () => void {
    this.handlers.add(h);
    return () => this.handlers.delete(h);
  }

  send(meta: object, body?: Uint8Array): void {
    if (!this.socket) throw new Error("not connected");
    this.socket.write(encodeFrame(meta, body));
  }

  list(): Promise<string[]> {
    return new Promise((resolve) => {
      this.pendingList.push(resolve);
      this.send({ type: "list" });
    });
  }

  /** Wait for the first frame of a given type matching an optional predicate. */
  await(type: string, predicate?: (m: any) => boolean): Promise<Frame> {
    return new Promise((resolve) => {
      const off = this.on((f) => {
        if (f.meta?.type === type && (!predicate || predicate(f.meta))) {
          off();
          resolve(f);
        }
      });
    });
  }

  close(): void {
    try {
      this.socket?.end();
    } catch {}
    this.socket = null;
  }
}
