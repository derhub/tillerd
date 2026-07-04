import { expect, test } from "bun:test";

import { buildBellNotification } from "./bell";

test("builds a bell notification attributed to its session and surface", () => {
  const event = buildBellNotification({
    sessionId: "sess-1",
    surfaceId: "surf-9",
    now: 1000,
    id: "fixed",
  });
  expect(event.category).toBe("surface-bell");
  expect(event.severity).toBe("info");
  expect(event.sessionId).toBe("sess-1");
  expect(event.surfaceId).toBe("surf-9");
  expect(event.ts).toBe(1000);
  expect(event.id).toBe("fixed");
});

test("names the session in the message when a label is known", () => {
  const event = buildBellNotification({
    sessionId: "sess-1",
    surfaceId: null,
    sessionLabel: "build",
  });
  expect(event.message).toContain("build");
});

test("falls back to a generic message without a label", () => {
  const event = buildBellNotification({ sessionId: null, surfaceId: null });
  expect(event.message.length).toBeGreaterThan(0);
});
