import { describe, expect, test } from "bun:test";

import { evaluateWhen, type ContextSnapshot, type WhenExpr } from "./when";

const ctx = (values: Record<string, boolean | string>): ContextSnapshot => values;

describe("evaluateWhen", () => {
  test("an absent expression is always available", () => {
    expect(evaluateWhen(undefined, ctx({}))).toBe(true);
  });

  test("an empty expression is always available", () => {
    expect(evaluateWhen([], ctx({ anything: true }))).toBe(true);
  });

  test("a bare key requires the key to be truthy", () => {
    expect(evaluateWhen(["hasSession"], ctx({ hasSession: true }))).toBe(true);
    expect(evaluateWhen(["hasSession"], ctx({ hasSession: false }))).toBe(false);
    expect(evaluateWhen(["hasSession"], ctx({}))).toBe(false);
  });

  test("a negated key requires the key to be falsy", () => {
    expect(evaluateWhen(["!terminalFocus"], ctx({ terminalFocus: false }))).toBe(true);
    expect(evaluateWhen(["!terminalFocus"], ctx({}))).toBe(true);
    expect(evaluateWhen(["!terminalFocus"], ctx({ terminalFocus: true }))).toBe(false);
  });

  test("terms are ANDed together", () => {
    const expr: WhenExpr = ["isDesktop", "!terminalFocus"];
    expect(evaluateWhen(expr, ctx({ isDesktop: true, terminalFocus: false }))).toBe(true);
    expect(evaluateWhen(expr, ctx({ isDesktop: true, terminalFocus: true }))).toBe(false);
    expect(evaluateWhen(expr, ctx({ isDesktop: false, terminalFocus: false }))).toBe(false);
  });

  test("a non-empty string key is truthy", () => {
    expect(evaluateWhen(["mode"], ctx({ mode: "edit" }))).toBe(true);
    expect(evaluateWhen(["mode"], ctx({ mode: "" }))).toBe(false);
  });
});
