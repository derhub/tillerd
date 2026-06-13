import { expect, test } from "bun:test";

import { distinctAttribute, filterRecords } from "./log-filter";
import type { LogRecord } from "./log-record";

function record(level: string, body: string, attributes: Record<string, unknown> = {}): LogRecord {
  return { timestamp: "t", level, body, attributes, resource: {}, raw: "" };
}

const RECORDS: LogRecord[] = [
  record("DEBUG", "starting up", { component: "daemon" }),
  record("INFO", "spawning pty", { component: "daemon", "session.id": "s1" }),
  record("ERROR", "socket refused", { component: "gate", "session.id": "s2" }),
];

test("level filter shows only records of the chosen level", () => {
  expect(filterRecords(RECORDS, { level: "INFO" }).map((r) => r.body)).toEqual(["spawning pty"]);
  expect(filterRecords(RECORDS, { level: "ERROR" }).map((r) => r.body)).toEqual(["socket refused"]);
});

test("free-text search matches body and attributes", () => {
  expect(filterRecords(RECORDS, { query: "pty" }).map((r) => r.body)).toEqual(["spawning pty"]);
  expect(filterRecords(RECORDS, { query: "gate" }).map((r) => r.body)).toEqual(["socket refused"]);
});

test("facet by component shows only matching records", () => {
  expect(filterRecords(RECORDS, { component: "gate" }).map((r) => r.body)).toEqual([
    "socket refused",
  ]);
});

test("facet by session.id shows only matching records", () => {
  expect(filterRecords(RECORDS, { sessionId: "s1" }).map((r) => r.body)).toEqual(["spawning pty"]);
});

test("distinctAttribute returns sorted unique values for a facet menu", () => {
  expect(distinctAttribute(RECORDS, "component")).toEqual(["daemon", "gate"]);
  expect(distinctAttribute(RECORDS, "session.id")).toEqual(["s1", "s2"]);
});
