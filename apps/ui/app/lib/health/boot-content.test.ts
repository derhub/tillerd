import { test, expect } from "bun:test";

import { bootContent } from "./boot-content";

test("booting before the grace delay shows nothing, never a skeleton flash", () => {
  expect(bootContent("booting", false)).toBe("blank");
});

test("booting past the grace delay shows a skeleton", () => {
  expect(bootContent("booting", true)).toBe("skeleton");
});

test("a ready orchestrator shows the content regardless of the grace flag", () => {
  expect(bootContent("ready", false)).toBe("content");
  expect(bootContent("ready", true)).toBe("content");
});

test("an errored orchestrator shows the content, not a skeleton or wall", () => {
  expect(bootContent("error", true)).toBe("content");
});
