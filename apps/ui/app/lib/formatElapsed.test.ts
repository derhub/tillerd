import { describe, expect, test } from "bun:test";

import { formatElapsed } from "./formatElapsed";

describe("formatElapsed", () => {
  test("reads as now under a minute", () => {
    const spawnedAt = 0;
    expect(formatElapsed(spawnedAt, 59_000)).toBe("now");
  });

  test("reads as whole minutes under an hour", () => {
    const spawnedAt = 0;
    expect(formatElapsed(spawnedAt, 3 * 60_000)).toBe("3m");
    expect(formatElapsed(spawnedAt, 59 * 60_000)).toBe("59m");
  });

  test("reads as hours and minutes at or beyond an hour", () => {
    const spawnedAt = 0;
    expect(formatElapsed(spawnedAt, 60 * 60_000)).toBe("1h 0m");
    expect(formatElapsed(spawnedAt, 72 * 60_000)).toBe("1h 12m");
  });

  test("never reports a negative elapsed time for clock skew", () => {
    const spawnedAt = 10_000;
    expect(formatElapsed(spawnedAt, 0)).toBe("now");
  });
});
