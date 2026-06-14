# 0031. Notifications persist in the orchestrator store

- Status: proposed
- Date: 2026-06-14
- Relates: ADR-0023 (data model), ADR-0022 (orchestrator owns the backend)

## Context

0.0.10 adds the notification center (roadmap). The roadmap scoped its history as in-memory,
"cleared on quit (no cross-session persistence required in 0.0.x)". During implementation the
requirement changed: notification history must survive application restarts.

This collides with the 0.0.6 architecture freeze, which lists the data model (ADR-0023) among
the seams that are frozen for the rest of 0.x — "additive on these seams, never a change to
them". Durable history needs somewhere durable to live.

Two shapes were considered:

- A new SQLite table in the orchestrator store, reached host-agnostically (mirrors the `setting`
  table, ADR-0023). The schema already evolves through an ordered migration chain
  (`migration_v1..v4`, `schema_version = migrations().len()`); `v2..v4` were added after `v1`,
  so the chain is designed to grow.
- A desktop-only side store (a JSON file or `tauri-plugin-store`). Rejected: it is not
  host-agnostic (the server host, expected before v1, would not see the same history) and it
  forks persistence away from the single `tillerd.db` source of truth (ADR-0023).

## Decision

Persist notifications in the orchestrator store via a new additive `migration_v5()` that creates
a `notification` table; nothing in the existing tables changes. The notification module exposes
`insert` / `list` / `prune` through the `Store` trait, mirroring the `setting` surface. The
desktop host tap writes each derived notification to the store and the in-app feed hydrates from
`list` on boot; retention prunes to keep the most recent 500.

This is the **additive** form of the freeze, not a change to a frozen seam: no existing table,
column, or wire shape is altered. It is recorded here because adding to the data model after the
0.0.6 freeze is a decision future 0.x data-model asks should be measured against — additive
migrations that leave existing tables untouched are permitted; rewrites of frozen tables are not.

It also **reverses** the roadmap's "cleared on quit" for 0.0.10; the roadmap line is superseded
by this ADR.

## Consequences

- Notification history survives restarts; the feed is durable and host-agnostic (the future
  server host inherits the same table and `Store` surface).
- The data-model seam now grows by one table; the freeze's intent (no rework of earlier
  versions) holds because existing tables are untouched.
- A new retention concern (unbounded growth) is handled by prune-on-insert (keep last 500).
- Notification derivation still lives in the desktop host adapter (additive, ADR-0022); only the
  storage moves into the orchestrator. A server host later taps its own signals into the same
  table.
