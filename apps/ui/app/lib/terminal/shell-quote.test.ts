import { expect, test } from "bun:test";

import { shellQuotePath } from "./shell-quote";

test("wraps a plain path in single quotes", () => {
  expect(shellQuotePath("/home/user/file.txt")).toBe("'/home/user/file.txt'");
});

test("keeps spaces safe inside the single quotes", () => {
  expect(shellQuotePath("/home/user/my file.txt")).toBe("'/home/user/my file.txt'");
});

test("escapes an embedded single quote with the '\\'' idiom", () => {
  expect(shellQuotePath("/home/user/it's a file")).toBe("'/home/user/it'\\''s a file'");
});

test("neutralises shell metacharacters by quoting", () => {
  expect(shellQuotePath("/a/$(rm -rf ~)/b")).toBe("'/a/$(rm -rf ~)/b'");
});

test("quotes an empty path as an empty string literal", () => {
  expect(shellQuotePath("")).toBe("''");
});
