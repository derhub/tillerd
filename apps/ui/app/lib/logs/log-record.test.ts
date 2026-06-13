import { expect, test } from "bun:test";

import { parseRecord } from "./log-record";

// Hand-authored synthetic tracing-subscriber JSON line (not captured output).
const LINE = JSON.stringify({
  timestamp: "2026-06-13T10:00:00.000000Z",
  level: "INFO",
  fields: { message: "spawning pty", "pty.pid": 42 },
  target: "tillerd::server",
  spans: [
    { "service.name": "tillerd-daemon", "service.version": "0.0.0", name: "service" },
    { "session.id": "s1", component: "daemon", name: "handle" },
  ],
});

test("maps timestamp, level, and body from the tracing JSON shape", () => {
  const r = parseRecord(LINE)!;
  expect(r.timestamp).toBe("2026-06-13T10:00:00.000000Z");
  expect(r.level).toBe("INFO");
  expect(r.body).toBe("spawning pty");
});

test("splits resource fields from attributes", () => {
  const r = parseRecord(LINE)!;
  expect(r.resource).toEqual({ "service.name": "tillerd-daemon", "service.version": "0.0.0" });
  expect(r.attributes).toEqual({ "pty.pid": 42, "session.id": "s1", component: "daemon" });
});

test("returns null for a blank or malformed line", () => {
  expect(parseRecord("")).toBeNull();
  expect(parseRecord("   ")).toBeNull();
  expect(parseRecord("{not json")).toBeNull();
});
