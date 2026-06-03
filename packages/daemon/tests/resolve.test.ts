import { test, expect, describe } from "bun:test";
import { satisfiesRange } from "../src/resolve";

describe("satisfiesRange", () => {
  test(">= equal version satisfies", () => {
    expect(satisfiesRange("1.2.3", ">=1.2.3")).toBe(true);
  });

  test(">= greater version satisfies", () => {
    expect(satisfiesRange("1.2.4", ">=1.2.3")).toBe(true);
  });

  test(">= lesser version does not satisfy", () => {
    expect(satisfiesRange("1.2.2", ">=1.2.3")).toBe(false);
  });

  test("> strictly greater satisfies", () => {
    expect(satisfiesRange("1.2.4", ">1.2.3")).toBe(true);
  });

  test("> equal does not satisfy", () => {
    expect(satisfiesRange("1.2.3", ">1.2.3")).toBe(false);
  });

  test("< lesser satisfies", () => {
    expect(satisfiesRange("1.2.2", "<1.2.3")).toBe(true);
  });

  test("< equal does not satisfy", () => {
    expect(satisfiesRange("1.2.3", "<1.2.3")).toBe(false);
  });

  test("<= equal satisfies", () => {
    expect(satisfiesRange("1.2.3", "<=1.2.3")).toBe(true);
  });

  test("^ same major higher patch satisfies", () => {
    expect(satisfiesRange("1.3.0", "^1.2.0")).toBe(true);
  });

  test("^ different major does not satisfy", () => {
    expect(satisfiesRange("2.0.0", "^1.0.0")).toBe(false);
  });

  test("~ same major and minor satisfies", () => {
    expect(satisfiesRange("1.2.5", "~1.2.3")).toBe(true);
  });

  test("~ different minor does not satisfy", () => {
    expect(satisfiesRange("1.3.0", "~1.2.3")).toBe(false);
  });

  test("* always satisfies", () => {
    expect(satisfiesRange("99.99.99", "*")).toBe(true);
  });

  test("no operator defaults to >=", () => {
    expect(satisfiesRange("1.2.3", "1.2.3")).toBe(true);
    expect(satisfiesRange("1.2.2", "1.2.3")).toBe(false);
  });

  test("invalid range returns true (lenient)", () => {
    expect(satisfiesRange("1.2.3", "not-a-range")).toBe(true);
  });
});
