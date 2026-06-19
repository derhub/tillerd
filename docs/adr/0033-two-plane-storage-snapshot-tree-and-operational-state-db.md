# 0033. Two-plane storage: snapshot tree + operational state.db

- Status: proposed
- Date: 2026-06-19

## Context

ADR-0023 made a single SQLite store (`tillerd.db`) authoritative for the whole product
tree. That conflates two kinds of data with opposite needs: domain (what the user authored —
workspaces, projects, sessions, panel layout, surface bindings) wants to be readable,
diffable, versionable, and portable; operational data (runtime status, indexes, baselines,
notifications, command lib) wants to be fast, machine-local, and disposable. A single opaque
DB serves neither well and blocks "store/version it however you like."

This supersedes ADR-0023's single-store boundary and the storage portion of 0.0.6. Pre-v1,
dev-only data is discarded (clean cutover, no migration).

## Decision

Split persistence into two planes, joined by stable `id`.

- **Domain plane = readable JSON snapshot tree.** `workspace -> project -> session`, one
  directory per entity (slug dir, stable `id` inside the file). Containment encodes hierarchy
  (no `workspace_id`/`project_id` fields); ordering via explicit `sortOrder`; atomic
  write-temp-rename; archive = move subtree to `.archive/`. `layout.json` holds the panel tree
  + surface bindings (`surface = { id, kind, placement, cwd }`).
- **Operational plane = `state.db` (SQLite), machine-local, regenerable.** Holds id->path
  index, per-entity baseline snapshots (base JSON + hash), command lib, notifications, `meta`,
  and typed runtime/view state. Keyed by `id`, never by path.
- **Split is per-concern, not per-entity.** A surface's binding lives in the domain plane; its
  runtime status lives in `state.db`. The stable `id` is the cross-plane join.
- **Storage-agnostic, machine-local pins.** Only the domain tree is relocatable/syncable.
  `state.db` (regenerable) and `vault.stronghold` (secret) are pinned machine-local and never
  sync. Profiles (`<profiles>/<name>/settings.jsonc`, one active, cascade) and templates
  (`<templates>/<slug>/template.jsonc`, library) are portable bundles, sibling to each other;
  a profile owns settings only. Secrets resolve from a Stronghold vault unlocked by an
  OS-keychain master password.
- **Slug is a cosmetic label.** Re-slugged on rename via atomic subtree move; collisions
  disambiguated (`foo` -> `foo-2`). The `id` is truth; the id->path index regenerates by
  scanning. URL intent carries the stable id (`?w=<id>`). `cwd` is relative to project root.
- **Zero watchers; lazy reconcile.** Files reconcile at startup (2-way: file vs base) + an
  explicit Re-sync (3-way: `merge3(base, file, ours)`; `ours` in-memory only). Flat files
  field-merge (disjoint auto, overlap -> prompt Override / Force-merge); `layout.json`
  tree-merges per node by stable node id. No file blocks boot — per-class fallback +
  notification.

## Consequences

- Domain is human-readable and versionable; the user chooses git/sync/backup. Operational
  state is disposable and machine-local — wiping `state.db` loses nothing (baselines
  regenerate to the file-at-boot because pending is in-memory only).
- Cross-plane references resolve only by `id`; anything path-keyed must go through the
  regenerable index. Worktree-as-entity is removed; a working directory is just `cwd`.
- A new conflict-resolution surface (per-node for layout, per-entity for flat) is required in
  the client (0.0.16/0.0.17). Glossary terms (profile, template) update in CONTEXT.md.
- Pairs with ADR-0034 (state-model-as-contract): `state.db`'s typed runtime columns are the
  state model's persistence and ship together in 0.0.15.
