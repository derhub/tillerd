//! Session-row retention.

import type { Database } from "bun:sqlite";

export const DEFAULT_SESSION_TTL_MS = 7 * 24 * 60 * 60 * 1000;

/** Resolve the retention window from env, falling back to the default for absent or invalid input. */
export function parseSessionTtlMs(raw: string | undefined): number {
  const parsed = Number(raw);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_SESSION_TTL_MS;
}

/**
 * Delete session rows older than the retention window so the table cannot grow without
 * bound across restarts. Returns the number of rows removed.
 */
export function pruneExpiredSessions(db: Database, nowMs: number, ttlMs: number): number {
  const cutoff = nowMs - ttlMs;
  return db.run("DELETE FROM sessions WHERE created_at < ?", [cutoff]).changes;
}
