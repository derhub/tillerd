## 1. Executive Summary

- **Status**: IMPROVABLE
- **Reviewer**: AI Architect (OpenSpec Reviewer)
- **Target Proposal**: snapshot-tree-storage (slice 1 of 0.0.15 — domain plane + drop worktree)
- **Summary**: The direction is correct and ADR-locked (ADR-0033/0034), and the alternatives
  fail the readable/diffable/portable/mergeable goal the ADR exists to satisfy — but the
  proposal must explicitly name four invariants that move from SQLite constraints to store-code
  responsibilities (placement-uniqueness, in-process write serialization, the Store-trait
  domain/operational split, and the in-memory test-double parity), or specs/design will drop them.

## 2. Research & Evidence

### Existing Logic / Utilities

- One monolithic `Store` trait owns both domain and operational methods —
  `crates/orchestrator/src/persistence/mod.rs:400`. Domain: workspace/project/session/surface
  CRUD + list + reorder + archive. Operational: command, setting, notification, launch_template,
  meta. The split the proposal needs is along an existing, clean line in this trait.
- `SqliteStore { conn: Mutex<Connection> }` — `persistence/sqlite.rs:16` — serializes all writes
  behind one mutex with a 5s `busy_timeout`. The file-tree store loses this for free and must
  re-provide in-process write serialization.
- `InMemoryStore` (`persistence/memory.rs`) mirrors every `Store` fn as the test double; tests
  construct it directly. A file-tree domain store needs an equivalent (tempdir-backed) to keep
  the unit layer fast and isolated.
- Migration runner: ordered `migrations() -> Vec<String>` v1–v7, `current_version()=len`,
  version in `meta` — `schema.rs:195`. Stays for the operational tables; domain DDL (the
  `workspace`/`project`/`session`/`surface` CREATE TABLEs and `worktree`) leaves.

### Related Patterns

- Atomic write-temp-rename is the standard durable-file idiom; the archive-as-subtree-move maps
  to a single `rename(2)`, which is **more** atomic than the prior transactional cascade
  soft-delete (`archive_project` cascading `deleted_at` across sessions+surfaces in a txn,
  `sqlite.rs`). No regression there.
- Containment-encodes-hierarchy (no `workspace_id`/`project_id` fields) matches ADR-0033's
  folder layout in ROADMAP.md; refs already use stable ids (`WorkspaceId`/`ProjectId`/etc. in
  `mod.rs`), so dropping the FK columns is mechanical.

### Potential Conflicts

- **`surface_session_placement` partial UNIQUE index** (`schema.rs`, added v4) enforces one live
  surface per `(session_id, placement)`. `add_surface_to_session` relies on it to raise
  `SurfaceConflict`. A file tree has no such constraint — the store must enforce placement
  uniqueness in code before writing `layout.json`.
- **Operational tables FK domain ids.** `setting(project_id)`, `notification(session_id/surface_id)`,
  `launch_template(project_id)`, `command` reference domain ids that, post-slice, live in files,
  not rows. The FKs to `project`/`session` break when those tables leave. Slice 1 must drop those
  FK clauses (ids become opaque text keys validated against the in-memory index) — captured under
  the Store-trait split, but the proposal does not yet say it.

### Code Evidence

```text
persistence/mod.rs:400   pub trait Store: Send + Sync   // domain + operational, one trait
persistence/sqlite.rs:16 struct SqliteStore { conn: Mutex<Connection> }  // write serialization
schema.rs (v4)           CREATE UNIQUE INDEX surface_session_placement
                           ON surface(session_id, placement)
                           WHERE deleted_at IS NULL AND placement IS NOT NULL
```

### Search Keywords Attempted

`Store trait`, `SqliteStore`, `InMemoryStore`, `migration_v`, `current_version`,
`surface_session_placement`, `worktree` / `git_worktree` / `worktree_id` / `WorktreeStep`,
`source_kind`, `deleted_at`, `layout_json`, `add_surface_to_session`, `SurfaceConflict`,
`tillerd_paths` / product store path. (Three fan-out investigators over the orchestrator crate,
SDK, and `openspec/specs/`.)

## 3. Alternative Analysis

| Approach | Pros | Cons | Complexity |
| :--- | :--- | :--- | :--- |
| **Proposed** (one-file-per-entity JSON tree) | Readable, diffable, per-entity atomic write, subtree archive/move, clean git history, ADR-locked | Full domain-CRUD rewrite; invariants move DB→code | High (but mandated) |
| **Alternative A** (keep SQLite, add JSON export/import) | No CRUD rewrite | Two sources of truth; export is a snapshot, not the live store; user file-edits aren't truth — defeats ADR-0033's goal | Medium |
| **Alternative B** (relocate the SQLite file into the data root) | Trivial | Binary, not human-readable/diffable; no 3-way merge possible (slices 3-4 dead on arrival) | Low |

### Conclusion

Both alternatives fail the load-bearing requirement — the domain must be the *live, editable,
mergeable* truth, not a derived export or an opaque binary. A/B also strand the later 0.0.15
slices (reconcile/merge3 need text files). The proposed tree is the correct and only fit; the
review's value is hardening *how* it preserves the invariants SQLite gave for free.

## 4. Feasibility Check

- [x] **Dependency Check**: No new crate needed — `std::fs` + `serde_json` (already in tree) cover
  atomic write-temp-rename, directory scan, and subtree move. `serde` types already exist for the
  domain structs. No watcher dependency (zero-watcher invariant).
- [x] **Performance Impact**: Boot-time tree scan to build the in-memory id→path index is
  O(entities); at pre-v1 scale (handful of workspaces/projects/sessions) negligible. Per-entity
  writes are small JSON blobs. No unbounded growth, no deepcopy hot path.
- [x] **Testability**: High, with the noted addition — a tempdir-backed domain store gives
  isolated, order-independent unit tests mirroring the current `InMemoryStore` pattern; the
  spec→test 1:1 mapping holds.

## 5. Detailed Verdict & Action Items

### Verdict

**IMPROVABLE.** The approach is sound, evidence-supported, and ADR-mandated; alternatives are
strictly worse against the stated goal. It is not OPTIMAL only because the proposal omits four
invariants that SQLite enforced implicitly and that must be explicit so specs/design (and the
contract tests) cover them.

### Action Items (exact edits to proposal.md)

1. **What Changes — Store-trait split**: state that the monolithic `Store` trait splits into a
   domain store (file-tree-backed) and an operational store (SQLite-backed), and that operational
   tables drop their FK clauses to domain ids (ids become opaque text keys validated via the
   in-memory index).
2. **What Changes — placement uniqueness**: add that the one-live-surface-per-`(session,
   placement)` invariant (previously the `surface_session_placement` partial unique index) is
   enforced in store code before writing `layout.json`, still raising `SurfaceConflict`.
3. **What Changes — write serialization**: add that the domain store provides in-process write
   serialization (replacing SQLite's connection mutex) so concurrent writes from multiple windows
   stay consistent (single embedded orchestrator process; an in-process lock suffices).
4. **Impact — testability**: note the tempdir-backed domain-store test double replacing
   `InMemoryStore` for domain coverage.

## 6. Review Metadata

- **Review Date**: 2026-06-19
- **Context Depth**: 3 fan-out investigators across `crates/orchestrator`, `packages/sdk`,
  `openspec/specs/` (persistence map, worktree touch-points, affected-spec classification);
  proposal.md + ADR-0033/0034 + ROADMAP 0.0.15 read directly.
- **Tools Used**: Grep, Read, Shell (Explore subagents)
