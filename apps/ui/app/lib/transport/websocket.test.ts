import { test, expect, describe } from "bun:test";
import { encodeFrame, FrameDecoder, type DaemonFrame } from "@athing/sdk";
import { WebSocketDaemonTransport, type WebSocketLike } from "./websocket";

class FakeWS implements WebSocketLike {
  binaryType = "blob";
  onopen: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: ((ev: unknown) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  readonly sent: Uint8Array[] = [];
  private readonly outDecoder = new FrameDecoder();

  send(data: ArrayBufferView): void {
    this.sent.push(new Uint8Array(data.buffer, data.byteOffset, data.byteLength));
  }
  close(): void {
    this.onclose?.({});
  }

  open(): void {
    this.onopen?.({});
  }
  /** Deliver a daemon→client frame to the transport, as the server byte bridge would. */
  deliver(meta: object, body?: Uint8Array): void {
    const buf = encodeFrame(meta, body);
    this.onmessage?.({ data: buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) });
  }
  /** Decode every frame the transport has sent so far. */
  sentFrames(): Array<{ meta: DaemonFrame | null; body: Uint8Array | null }> {
    const out: Array<{ meta: DaemonFrame | null; body: Uint8Array | null }> = [];
    for (const chunk of this.sent.splice(0)) {
      for (const { meta, body } of this.outDecoder.push(chunk)) {
        out.push({ meta: meta as DaemonFrame, body });
      }
    }
    return out;
  }
}

describe("WebSocketDaemonTransport", () => {
  test("sends hello on open and resolves connect on hello-ack", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    const [hello] = ws.sentFrames();
    expect(hello.meta).toMatchObject({ type: "hello", versions: [1] });
    ws.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;
  });

  test("rejects connect when an error frame precedes hello-ack", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    ws.deliver({ type: "error", code: "Boom", message: "nope" });
    await expect(connected).rejects.toThrow(/nope/);
  });

  test("dispatches session frames to the matching subscriber with raw body", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    ws.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;

    const received: Uint8Array[] = [];
    t.subscribe("s1", (_f, body) => body && received.push(body));
    const payload = new Uint8Array([1, 2, 3, 255, 0]);
    ws.deliver({ type: "data", sessionId: "s1", bodyLen: payload.length }, payload);

    expect(received).toHaveLength(1);
    expect([...received[0]!]).toEqual([1, 2, 3, 255, 0]);
  });

  test("does not deliver another session's frames to a subscriber", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    ws.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;

    let hits = 0;
    t.subscribe("s1", () => hits++);
    ws.deliver({ type: "data", sessionId: "s2", bodyLen: 1 }, new Uint8Array([9]));
    expect(hits).toBe(0);
  });

  test("list() resolves on the list-ack ids", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    ws.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;

    const p = t.list();
    const sent = ws.sentFrames();
    expect(sent.at(-1)!.meta).toMatchObject({ type: "list" });
    ws.deliver({ type: "list-ack", ids: ["a", "b"] });
    expect(await p).toEqual(["a", "b"]);
  });

  test("onClose fires and pending list() drains on socket close", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    ws.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;

    let closed = false;
    t.onClose(() => (closed = true));
    const p = t.list();
    ws.close();
    expect(closed).toBe(true);
    expect(await p).toEqual([]);
  });

  test("send throws once disconnected", async () => {
    let ws!: FakeWS;
    const t = new WebSocketDaemonTransport("ws://x", () => (ws = new FakeWS()));
    const connected = t.connect();
    ws.open();
    ws.deliver({ type: "hello-ack", version: 1, daemonVersion: "0.0.1" });
    await connected;
    t.disconnect();
    expect(() => t.send({ type: "list" })).toThrow(/not connected/);
  });
});
