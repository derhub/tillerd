import { expect, test } from "bun:test";

import type { LogRecord } from "./log-record";

import { distinctAttribute, distinctService, filterRecords } from "./log-filter";

function record(
  level: string,
  body: string,
  attributes: Record<string, unknown> = {},
  resource: Record<string, unknown> = {},
): LogRecord {
  return { timestamp: "t", level, body, attributes, resource, raw: "" };
}

const RECORDS: LogRecord[] = [
  record("DEBUG", "starting up", { component: "daemon" }, { "service.name": "tillerd-daemon" }),
  record(
    "INFO",
    "spawning pty",
    { component: "daemon", "session.id": "s1" },
    { "service.name": "tillerd-daemon" },
  ),
  record(
    "ERROR",
    "socket refused",
    { component: "gate", "session.id": "s2" },
    { "service.name": "tillerd-gate" },
  ),
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

test("facet by service shows only that service's records", () => {
  expect(filterRecords(RECORDS, { service: "tillerd-gate" }).map((r) => r.body)).toEqual([
    "socket refused",
  ]);
});

test("distinctAttribute returns sorted unique values for a facet menu", () => {
  expect(distinctAttribute(RECORDS, "component")).toEqual(["daemon", "gate"]);
  expect(distinctAttribute(RECORDS, "session.id")).toEqual(["s1", "s2"]);
});

test("distinctService returns sorted unique service names for the service facet", () => {
  expect(distinctService(RECORDS)).toEqual(["tillerd-daemon", "tillerd-gate"]);
});
