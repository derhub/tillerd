import { test, expect, describe } from "bun:test";
import { ReplayBuffer } from "../src/replay-buffer";

describe("ReplayBuffer", () => {
  test("snapshot is empty initially", () => {
    const b = new ReplayBuffer();
    expect(b.snapshot()).toHaveLength(0);
  });

  test("snapshot contains pushed chunks in order", () => {
    const b = new ReplayBuffer();
    const a = new Uint8Array([1, 2, 3]);
    const c = new Uint8Array([4, 5]);
    b.push(a);
    b.push(c);
    const snap = b.snapshot();
    expect(snap).toHaveLength(2);
    expect(snap[0]).toEqual(a);
    expect(snap[1]).toEqual(c);
  });

  test("snapshot returns a copy — mutations do not affect buffer", () => {
    const b = new ReplayBuffer();
    b.push(new Uint8Array([1]));
    const snap = b.snapshot();
    snap.push(new Uint8Array([99]));
    expect(b.snapshot()).toHaveLength(1);
  });

  test("evicts oldest chunks when capacity exceeded", () => {
    const b = new ReplayBuffer();
    const CAPACITY = 64 * 1024;
    // Fill to just under capacity
    const chunk = new Uint8Array(32 * 1024); // 32 KB
    b.push(chunk);
    b.push(chunk);
    // Now add another 32 KB — total 96 KB, first chunk evicted
    b.push(chunk);
    const snap = b.snapshot();
    // 32 KB * 2 chunks = 64 KB (first 32 KB dropped)
    const total = snap.reduce((s, c) => s + c.length, 0);
    expect(total).toBeLessThanOrEqual(CAPACITY);
  });

  test("chunk larger than capacity is evicted immediately (buffer stays empty)", () => {
    const b = new ReplayBuffer();
    const CAPACITY = 64 * 1024;
    // A chunk larger than the total capacity is pushed then evicted by the while-loop.
    b.push(new Uint8Array(CAPACITY + 1));
    expect(b.snapshot()).toHaveLength(0);
  });

  test("preserves byte content through push→snapshot round-trip", () => {
    const b = new ReplayBuffer();
    const original = new Uint8Array([0x1b, 0x5b, 0x33, 0x31, 0x6d]); // ESC[31m
    b.push(original);
    const snap = b.snapshot();
    expect(snap[0]).toEqual(original);
  });
});
