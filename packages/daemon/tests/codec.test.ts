import { test, expect, describe } from "bun:test";
import { encodeFrame, FrameDecoder } from "../src/protocol/codec";

describe("encodeFrame / FrameDecoder", () => {
  test("round-trip: frame without binary body", () => {
    const meta = { type: "hello", versions: [1] };
    const encoded = encodeFrame(meta);
    const decoder = new FrameDecoder();
    const frames = decoder.push(encoded);
    expect(frames).toHaveLength(1);
    expect(frames[0]!.meta).toEqual(meta);
    expect(frames[0]!.body).toBeNull();
  });

  test("round-trip: frame with binary body", () => {
    const meta = { type: "data", sessionId: "s1", bodyLen: 5 };
    const body = new Uint8Array([1, 2, 3, 4, 5]);
    const encoded = encodeFrame(meta, body);
    const decoder = new FrameDecoder();
    const frames = decoder.push(encoded);
    expect(frames).toHaveLength(1);
    expect(frames[0]!.meta).toEqual(meta);
    expect(Array.from(frames[0]!.body!)).toEqual([1, 2, 3, 4, 5]);
  });

  test("multiple frames in one push", () => {
    const f1 = encodeFrame({ type: "ping" });
    const f2 = encodeFrame({ type: "pong" });
    const combined = Buffer.concat([f1, f2]);
    const decoder = new FrameDecoder();
    const frames = decoder.push(combined);
    expect(frames).toHaveLength(2);
    expect((frames[0]!.meta as { type: string }).type).toBe("ping");
    expect((frames[1]!.meta as { type: string }).type).toBe("pong");
  });

  test("incomplete frame held across two pushes", () => {
    const encoded = encodeFrame({ type: "hello" });
    const half = encoded.length >>> 1;
    const decoder = new FrameDecoder();
    const first = decoder.push(encoded.subarray(0, half));
    expect(first).toHaveLength(0);
    const second = decoder.push(encoded.subarray(half));
    expect(second).toHaveLength(1);
    expect((second[0]!.meta as { type: string }).type).toBe("hello");
  });

  test("body containing 0x0A bytes is preserved verbatim", () => {
    const body = new Uint8Array([0x0a, 0x0a, 0x41]);
    const encoded = encodeFrame({ type: "data" }, body);
    const decoder = new FrameDecoder();
    const frames = decoder.push(encoded);
    expect(Array.from(frames[0]!.body!)).toEqual([0x0a, 0x0a, 0x41]);
  });
});
