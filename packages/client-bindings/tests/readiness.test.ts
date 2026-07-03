import { afterEach, describe, expect, test } from "bun:test";

import { isReady, setReady, whenReady } from "../src/readiness";

afterEach(() => setReady(false));

describe("readiness state", () => {
  test("isReady is false before setReady is called", () => {
    expect(isReady()).toBe(false);
  });

  test("setReady(true) marks ready", () => {
    setReady(true);
    expect(isReady()).toBe(true);
  });

  test("setReady(false) clears the ready state", () => {
    setReady(true);
    setReady(false);
    expect(isReady()).toBe(false);
  });
});

describe("whenReady", () => {
  test("stays pending until setReady(true) is called", async () => {
    let resolved = false;
    const p = whenReady().then(() => {
      resolved = true;
    });
    await Promise.resolve();
    expect(resolved).toBe(false);
    setReady(true);
    await p;
    expect(resolved).toBe(true);
  });

  test("resolves true once setReady(true) is called", async () => {
    const p = whenReady();
    setReady(true);
    expect(await p).toBe(true);
  });

  test("resolves false when setReady(false) is called (web host signal)", async () => {
    const p = whenReady();
    setReady(false);
    expect(await p).toBe(false);
  });

  test("a call after reset returns a fresh pending promise", async () => {
    const p1 = whenReady();
    setReady(true);
    expect(await p1).toBe(true);

    setReady(false);

    const p2 = whenReady();
    setReady(true);
    expect(await p2).toBe(true);
  });
});
