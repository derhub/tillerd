import { test, expect, describe, afterEach } from "bun:test";

import { randomId, hasSecureCrypto } from "./web-crypto";

const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const realCrypto = globalThis.crypto;

afterEach(() => {
  Object.defineProperty(globalThis, "crypto", { value: realCrypto, configurable: true });
});

function stubCrypto(value: unknown): void {
  Object.defineProperty(globalThis, "crypto", { value, configurable: true });
}

describe("randomId", () => {
  test("uses crypto.randomUUID when present", () => {
    expect(randomId()).toMatch(UUID_V4);
    expect(hasSecureCrypto()).toBe(true);
  });

  test("falls back to getRandomValues with a valid v4 shape", () => {
    stubCrypto({
      getRandomValues: (a: Uint8Array) => {
        for (let i = 0; i < a.length; i++) a[i] = i;
        return a;
      },
    });
    expect(hasSecureCrypto()).toBe(false);
    expect(randomId()).toMatch(UUID_V4);
  });

  test("falls back to a weak source when crypto is absent", () => {
    stubCrypto(undefined);
    expect(randomId()).toMatch(UUID_V4);
  });
});
