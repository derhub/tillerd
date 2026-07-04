import type { NotificationWire } from "@tillerd/client-bindings";

import { expect, test } from "bun:test";

import { boundedPrepend, countsAsUnread, notificationHeading } from "./store";

function ev(id: string, over: Partial<NotificationWire> = {}): NotificationWire {
  return {
    id,
    category: "surface-stopped",
    severity: "info",
    message: `m${id}`,
    ts: Number(id) || 0,
    ...over,
  };
}

test("boundedPrepend puts the newest first", () => {
  const out = boundedPrepend([ev("1")], ev("2"));
  expect(out.map((e) => e.id)).toEqual(["2", "1"]);
});

test("boundedPrepend de-dupes by id", () => {
  const out = boundedPrepend([ev("1"), ev("2")], ev("1"));
  expect(out.map((e) => e.id)).toEqual(["1", "2"]);
});

test("boundedPrepend trims the oldest beyond the bound", () => {
  const items = [ev("0"), ev("1"), ev("2")];
  const out = boundedPrepend(items, ev("new"), 3);
  expect(out).toHaveLength(3);
  expect(out[0].id).toBe("new");
  expect(out.map((e) => e.id)).not.toContain("2");
});

test("notificationHeading uses the title when present", () => {
  expect(notificationHeading(ev("1", { title: "Custom" }))).toBe("Custom");
});

test("notificationHeading falls back to a category label", () => {
  expect(notificationHeading(ev("1", { category: "service-down" }))).toBe("Service down");
});

test("notificationHeading handles an unrecognised category", () => {
  expect(notificationHeading(ev("1", { category: "future-thing" }))).toBe("Notification");
});

test("countsAsUnread badges warnings and errors, not ambient info", () => {
  expect(countsAsUnread(ev("1", { severity: "error" }))).toBe(true);
  expect(countsAsUnread(ev("1", { severity: "warning" }))).toBe(true);
  expect(countsAsUnread(ev("1", { severity: "info" }))).toBe(false);
});
