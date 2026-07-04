import { expect, test } from "bun:test";

import { sessionDisplayName, terminalTitle } from "./panelTitle";

test("terminalTitle joins name and surface kind before the PTY spawns", () => {
  expect(terminalTitle("build", null, 0)).toBe("build · Terminal");
});

test("terminalTitle appends elapsed time once the surface has spawned", () => {
  expect(terminalTitle("build", 0, 0)).toBe("build · Terminal · now");
});

test("terminalTitle drops the leading separator when the name is empty", () => {
  expect(terminalTitle("", null, 0)).toBe("Terminal");
  expect(terminalTitle("", 0, 0)).toBe("Terminal · now");
});

test("sessionDisplayName uses the title when present", () => {
  expect(sessionDisplayName("My session", "eb9d1e1f-1234")).toBe("My session");
});

test("sessionDisplayName falls back to a short id slice for a blank title", () => {
  expect(sessionDisplayName("", "eb9d1e1f-1234")).toBe("eb9d1e1f");
  expect(sessionDisplayName("   ", "eb9d1e1f-1234")).toBe("eb9d1e1f");
  expect(sessionDisplayName(undefined, "eb9d1e1f-1234")).toBe("eb9d1e1f");
});

test("sessionDisplayName falls back to a generic label without an id", () => {
  expect(sessionDisplayName("", null)).toBe("Session");
});
