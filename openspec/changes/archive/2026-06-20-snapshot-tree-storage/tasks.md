## 1. Drop worktree entity and provisioning (mechanical, first)

- [x] 1.1 Remove the `worktree` table DDL, the `surface.worktree_id` column + its FK, and
  `'git_worktree'` from the `project.source_kind` CHECK (`persistence/schema.rs`); remove
  `Worktree`/`WorktreeId`/`NewWorktree`, the `SourceKind::GitWorktree` variant, the three
  `Store` worktree methods + their sqlite/memory impls + `row_to_worktree` + worktree tests
  (`persistence/{mod.rs,sqlite.rs,memory.rs}`); drop `worktree_id` from `Surface`/`NewSurface`
  and the surface INSERT/SELECT/row-mapper.
- [x] 1.2 Remove the launch worktree step: delete `launch/worktree.rs` + `pub mod worktree`
  (`launch/mod.rs`), `WorktreeStep` + `LaunchItem.worktree` (`launch/spec.rs`),
  `run_worktree_step` + the worktree match block + imports (`launch/executor.rs`), the two
  worktree `error.rs` variants, and `worktree_id: None` literals in `surface/api.rs`.
- [x] 1.3 Drop `"git_worktree"` from the SDK `SourceKind` union (`packages/sdk/src/orchestrator/workspace.ts`).

## 2. Operational store + trait split

- [x] 2.1 Collapse the SQLite schema to operational tables only (meta/command/setting/
  notification/launch_template), dropping domain DDL and operational→domain FK clauses
  (`persistence/schema.rs`); ids become opaque text keys.
- [x] 2.2 Split `Store` into `OperationalStore` (SQLite) + `DomainStore` (trait) with a `Store`
  facade composing both; move operational methods to `OperationalStore`, keep the existing
  `SqliteStore` backing it (`persistence/mod.rs`, `sqlite.rs`).

## 3. File-tree DomainStore (TDD — one unit test per spec scenario, red→green→refactor)

- [x] 3.1 New `persistence/tree/` module: a `TempDir`-backed `DomainStore` impl with
  `RwLock<TreeState>` (id→path index + boot scan), atomic write-temp-rename, `create_dir_all`.
- [x] 3.2 Workspace/project/session CRUD + list, persisting `workspace.json`/`project.json`/
  `session.json` with stable `id` + `sortOrder`, containment-derived hierarchy; reorder by
  `sortOrder`; seed Default workspace + Unfiled project on empty tree.
- [x] 3.3 Slug derivation + sibling collision suffix (`foo`→`foo-2`); rename = re-slug + atomic
  subtree move keeping the `id`; index follows the move.
- [x] 3.4 Archive = subtree move to `.archive/` (project carries its sessions); hard-delete
  removes the archived subtree.
- [x] 3.5 `layout.json` panel tree + surface bindings (`{id,kind,placement,cwd}`, `cwd` relative
  to project root); placement uniqueness enforced in code raising `SurfaceConflict`;
  `list_resumable_surfaces` reads live bindings from the tree.

## 4. Wire-up

- [x] 4.1 `tillerd_paths`: replace the single product-store file path with a data-root directory
  builder (default `<tillerd_dir>/data`); compose the `Store` facade (DomainStore over data root
  + OperationalStore over `tillerd.db`) at orchestrator init; route domain API methods to
  `DomainStore`, keeping request/response signatures stable.
- [x] 4.2 Boot-scan minimal resilience: a malformed domain file skips mounting that entity + logs
  (full reconcile is slice 4).

## 5. Verify gate (fix-all)

- [x] 5.1 Run the full verify suite to green: `fmt`, `clippy --all-targets --locked -D warnings`,
  unit tests (every spec scenario across all 12 capability deltas, 1:1), and the affected e2e
  (boot/create/resume); confirm spec↔test 1:1 (`openspec verify`), fix any gap, re-run until
  clean. Mark breaking commits `!`.
