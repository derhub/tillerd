import { test, expect, describe, beforeEach } from "bun:test";
import { Database } from "bun:sqlite";
import { pruneExpiredSessions, parseSessionTtlMs, DEFAULT_SESSION_TTL_MS } from "../src/sessions";

function makeDb(): Database {
  const db = new Database(":memory:");
  db.run(`CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    cwd TEXT NOT NULL,
    created_at INTEGER NOT NULL
  )`);
  return db;
}

describe("pruneExpiredSessions", () => {
  const now = 1_000_000_000_000;
  const ttl = 1000;
  let db: Database;

  beforeEach(() => {
    db = makeDb();
  });

  test("removes rows older than the retention window", () => {
    db.run("INSERT INTO sessions (id, cwd, created_at) VALUES (?, ?, ?)", [
      "old",
      "/x",
      now - ttl - 1,
    ]);
    const removed = pruneExpiredSessions(db, now, ttl);
    expect(removed).toBe(1);
  });

  test("keeps rows within the retention window", () => {
    db.run("INSERT INTO sessions (id, cwd, created_at) VALUES (?, ?, ?)", ["fresh", "/x", now - 1]);
    pruneExpiredSessions(db, now, ttl);
    const remaining = db.query("SELECT id FROM sessions").all() as Array<{ id: string }>;
    expect(remaining).toHaveLength(1);
  });
});

describe("parseSessionTtlMs", () => {
  test("uses the default when the env value is absent", () => {
    expect(parseSessionTtlMs(undefined)).toBe(DEFAULT_SESSION_TTL_MS);
  });

  test("uses the default when the env value is not a positive number", () => {
    expect(parseSessionTtlMs("nonsense")).toBe(DEFAULT_SESSION_TTL_MS);
  });

  test("honours a positive override", () => {
    expect(parseSessionTtlMs("5000")).toBe(5000);
  });
});
