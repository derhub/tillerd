import { test, expect, describe } from "bun:test";
import { encodeFrame, FrameDecoder } from "@tillerd/sdk";

const enc = new TextEncoder();

describe("wire codec — round-trip", () => {
  test("frame without body", () => {
    const dec = new FrameDecoder();
    const frames = dec.push(encodeFrame({ type: "list" }));
    expect(frames).toHaveLength(1);
    expect(frames[0]!.meta).toEqual({ type: "list" });
    expect(frames[0]!.body).toBeNull();
  });

  test("frame with binary body — body is a Uint8Array, bytes preserved", () => {
    const dec = new FrameDecoder();
    const body = new Uint8Array([0, 1, 2, 0x0a, 255, 254]);
    const frames = dec.push(encodeFrame({ type: "data", sessionId: "s" }, body));
    expect(frames).toHaveLength(1);
    expect(frames[0]!.meta).toEqual({ type: "data", sessionId: "s" });
    expect(frames[0]!.body).toBeInstanceOf(Uint8Array);
    expect(Array.from(frames[0]!.body!)).toEqual(Array.from(body));
  });

  test("body containing the 0x0a separator survives (split on first newline only)", () => {
    const dec = new FrameDecoder();
    const body = enc.encode("line1\nline2\n");
    const frames = dec.push(encodeFrame({ type: "data" }, body));
    expect(Array.from(frames[0]!.body!)).toEqual(Array.from(body));
  });
});

describe("wire codec — framing", () => {
  test("multiple frames in one chunk", () => {
    const a = encodeFrame({ type: "a" });
    const b = encodeFrame({ type: "b" }, new Uint8Array([9]));
    const merged = new Uint8Array(a.length + b.length);
    merged.set(a, 0);
    merged.set(b, a.length);

    const frames = new FrameDecoder().push(merged);
    expect(frames.map((f) => (f.meta as { type: string }).type)).toEqual(["a", "b"]);
    expect(Array.from(frames[1]!.body!)).toEqual([9]);
  });

  test("frame split across two pushes is buffered until complete", () => {
    const dec = new FrameDecoder();
    const full = encodeFrame({ type: "split" }, new Uint8Array([1, 2, 3]));
    const cut = 3;

    expect(dec.push(full.subarray(0, cut))).toHaveLength(0);
    const frames = dec.push(full.subarray(cut));
    expect(frames).toHaveLength(1);
    expect(frames[0]!.meta).toEqual({ type: "split" });
    expect(Array.from(frames[0]!.body!)).toEqual([1, 2, 3]);
  });

  test("encodeFrame returns a Uint8Array (not a runtime-specific buffer)", () => {
    const out = encodeFrame({ type: "x" });
    expect(out).toBeInstanceOf(Uint8Array);
  });

  test("byte-identical encoding is deterministic", () => {
    const a = encodeFrame({ type: "data", sessionId: "s" }, new Uint8Array([1, 2]));
    const b = encodeFrame({ type: "data", sessionId: "s" }, new Uint8Array([1, 2]));
    expect(Array.from(a)).toEqual(Array.from(b));
  });
});
