import { expect, test } from "bun:test";

import { isOpenableUrl } from "./open-external";

test("accepts http and https links", () => {
  expect(isOpenableUrl("http://example.com")).toBe(true);
  expect(isOpenableUrl("https://example.com/path?q=1")).toBe(true);
});

test("accepts mailto links", () => {
  expect(isOpenableUrl("mailto:someone@example.com")).toBe(true);
});

test("rejects file and javascript schemes", () => {
  expect(isOpenableUrl("file:///etc/passwd")).toBe(false);
  expect(isOpenableUrl("javascript:alert(1)")).toBe(false);
});

test("rejects a non-url string", () => {
  expect(isOpenableUrl("not a url")).toBe(false);
});
