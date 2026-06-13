import { expect, test } from "bun:test";

import type { LogFileInfo, LogSource } from "../transport/log-source";
import { LogTail } from "./log-tail";

function rec(ts: string, msg: string): string {
  return JSON.stringify({
    timestamp: ts,
    level: "INFO",
    fields: { message: msg },
    spans: [{ "service.name": "svc", name: "service" }],
  });
}

const enc = (s: string): number => new TextEncoder().encode(s).length;

class FakeSource implements LogSource {
  private files = new Map<string, Uint8Array>();

  put(path: string, content: string): void {
    this.files.set(path, new TextEncoder().encode(content));
  }

  append(path: string, more: string): void {
    const cur = this.files.get(path) ?? new Uint8Array();
    const add = new TextEncoder().encode(more);
    const next = new Uint8Array(cur.length + add.length);
    next.set(cur);
    next.set(add, cur.length);
    this.files.set(path, next);
  }

  list(): Promise<LogFileInfo[]> {
    return Promise.resolve(
      [...this.files.entries()].map(([path, b]) => ({ name: path, path, size: b.length })),
    );
  }

  size(path: string): Promise<number | null> {
    const b = this.files.get(path);
    return Promise.resolve(b ? b.length : null);
  }

  read(path: string, offset: number, length: number): Promise<Uint8Array> {
    const b = this.files.get(path) ?? new Uint8Array();
    return Promise.resolve(b.slice(offset, offset + length));
  }
}

test("shows recent history on open", async () => {
  const src = new FakeSource();
  src.put("/logs/a.log", `${rec("t1", "one")}\n${rec("t2", "two")}\n`);
  const tail = new LogTail(src);
  const records = await tail.refresh();
  expect(records.map((r) => r.body)).toEqual(["one", "two"]);
});

test("new record appears on the next refresh", async () => {
  const src = new FakeSource();
  src.put("/logs/a.log", `${rec("t1", "one")}\n`);
  const tail = new LogTail(src);
  await tail.refresh();
  src.append("/logs/a.log", `${rec("t2", "two")}\n`);
  const records = await tail.refresh();
  expect(records.map((r) => r.body)).toEqual(["one", "two"]);
});

test("a partial trailing line is withheld until its newline arrives", async () => {
  const src = new FakeSource();
  src.put("/logs/a.log", `${rec("t1", "one")}\n`);
  const tail = new LogTail(src);
  await tail.refresh();

  src.append("/logs/a.log", rec("t2", "two")); // no newline yet
  expect((await tail.refresh()).map((r) => r.body)).toEqual(["one"]);

  src.append("/logs/a.log", "\n"); // completes the line
  expect((await tail.refresh()).map((r) => r.body)).toEqual(["one", "two"]);
});

test("merges records across files in timestamp order", async () => {
  const src = new FakeSource();
  src.put(
    "/logs/a.log",
    `${rec("2026-01-01T00:00:01Z", "a1")}\n${rec("2026-01-01T00:00:03Z", "a3")}\n`,
  );
  src.put("/logs/b.log", `${rec("2026-01-01T00:00:02Z", "b2")}\n`);
  const tail = new LogTail(src);
  const records = await tail.refresh();
  expect(records.map((r) => r.body)).toEqual(["a1", "b2", "a3"]);
});

test("loadOlder prepends earlier records", async () => {
  const a = `${rec("t1", "aaa")}\n`;
  const b = `${rec("t2", "bbb")}\n`;
  const c = `${rec("t3", "ccc")}\n`;
  const src = new FakeSource();
  src.put("/logs/x.log", a + b + c);
  const tail = new LogTail(src, { backfillBytes: enc(c), olderChunkBytes: enc(b) });

  expect((await tail.refresh()).map((r) => r.body)).toEqual(["ccc"]);
  expect((await tail.loadOlder("/logs/x.log")).map((r) => r.body)).toEqual(["bbb", "ccc"]);
});

test("loadOlderAll loads earlier records across every tracked file", async () => {
  const a = `${rec("t1", "aaa")}\n`;
  const b = `${rec("t2", "bbb")}\n`;
  const src = new FakeSource();
  src.put("/logs/x.log", a + b);
  src.put("/logs/y.log", a + b);
  const tail = new LogTail(src, { backfillBytes: enc(b), olderChunkBytes: enc(a) });

  await tail.refresh();
  const all = await tail.loadOlderAll();
  expect(all.filter((r) => r.body === "aaa").length).toBe(2);
});
