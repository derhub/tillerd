import { test, expect, describe } from "bun:test";

import { uuid, hasSecureCrypto } from "./web-crypto";

const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

describe("uuid", () => {
  test("generates a valid UUID v4 string", () => {
    expect(uuid()).toMatch(UUID_V4);
    expect(hasSecureCrypto()).toBe(true);
  });
});
