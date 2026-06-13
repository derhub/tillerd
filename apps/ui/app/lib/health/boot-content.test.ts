import { test, expect } from "bun:test";

import { bootContent } from "./boot-content";

// Scenario: No skeleton flash on fast resolve
test("booting before the grace delay shows nothing, never a skeleton flash", () => {
  expect(bootContent("booting", false)).toBe("blank");
});

// Scenario: Skeleton after the grace delay while slow
test("booting past the grace delay shows a skeleton", () => {
  expect(bootContent("booting", true)).toBe("skeleton");
});

// Scenario: Content replaces skeleton when ready
test("a ready orchestrator shows the content regardless of the grace flag", () => {
  expect(bootContent("ready", false)).toBe("content");
  expect(bootContent("ready", true)).toBe("content");
});

// A boot failure renders the content (shell stays usable); the indicator flags it.
test("an errored orchestrator shows the content, not a skeleton or wall", () => {
  expect(bootContent("error", true)).toBe("content");
});
