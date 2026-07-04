import { expect, test } from "bun:test";

import { shouldConfirmPaste } from "./paste";

test("confirms a multi-line paste when the setting is enabled", () => {
  expect(shouldConfirmPaste("line one\nline two", true)).toBe(true);
});

test("treats a carriage return as multi-line", () => {
  expect(shouldConfirmPaste("line one\rline two", true)).toBe(true);
});

test("does not confirm single-line content", () => {
  expect(shouldConfirmPaste("just one line", true)).toBe(false);
});

test("never confirms when the setting is disabled", () => {
  expect(shouldConfirmPaste("line one\nline two", false)).toBe(false);
});

test("does not confirm an empty clipboard", () => {
  expect(shouldConfirmPaste("", true)).toBe(false);
});
