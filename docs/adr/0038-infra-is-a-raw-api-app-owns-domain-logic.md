# 0038. Infra is a raw API; app owns all domain logic

- Status: accepted, amends ADR-0036
- Amends: ADR-0036
- Date: 2026-06-22

## Context

ADR-0036 de-abstracted storage into pure `entities/`, per-entity `infra/` repos plus the surface
runtime, and a CQS `app/` layer. It located persistence in infra but did not draw a hard line
against domain logic living there. In practice rules leaked into infra: stores gatekeep invariants
(prebuilt-immutable), repos encode retention and scope-precedence policies (settings, and the
mirror-image keybinding resolve/merge/shadowing), the surface runtime rejects non-terminal kinds,
repos hardcode initial state and semantic ordering (surface live-first, session pinned-float), and a
repo normalizes a domain field on the way out. An audit of every file under
`crates/orchestrator/src/infra/` found eight such leaks; the remainder is clean raw I/O. The runtime
also still carries one structural abstraction — the `SurfaceRuntime` port (`Arc<dyn>`) named by
ADR-0036 — over a single real I/O target (the daemon-socket client).

Two structural rules now in flight — `entities-app-or-infra-only` and `infra-only-in-app` — push the same
model from the other side: only `app/` and the bootstrap (`boot.rs`, `context.rs`) may name `crate::infra`,
and only `app/`, `infra/`, and the bootstrap may name `crate::entities` (infra names entities solely for
`Row <-> Entity` column mapping; the bootstrap is the composition root and may name any layer). The naming
boundary is already green; the rules flip from `warning` to `error` once the domain-logic leaks also leave
infra.

## Decision

Draw the line explicitly and move the leaks.

- **infra/ is a raw API.** It may only execute and bind database statements, map columns to and from
  entity fields, open/read/write sockets, encode/decode wire frames, and read/write files. It holds
  no business invariant, no retention/precedence/ordering/capability policy, no initial-state
  decision, and no multi-step "load, apply a rule, persist" sequence. A repo offers bare
  get/list/create/update/delete; the surface runtime offers spawn/input/resize and a decoded-frame
  source — kind-agnostic.

- **The runtime is a concrete raw client, not a port.** `infra/runtime/` is renamed
  `infra/daemon_pty_api/`; the `SurfaceRuntime` trait + `Arc<dyn>` dispatch are removed in favour of a
  concrete `DaemonPtyApi` (the daemon-socket client). The composition root holds a `Runtime` enum
  `{ Daemon(DaemonPtyApi), Fake(FakeRuntime) }` and dispatches statically — consistent with ADR-0035's
  original "enum dispatch, no trait objects." The output boundary `SurfaceEventSink` (a genuine
  multi-renderer port) stays a trait. The `daemon-pty-client` crate (raw wire codec, below infra) is
  untouched.

- **app/ owns all domain logic and is the sole integrator of entities + infra.** A use-case handler
  loads through raw infra, applies entity rules, and persists through raw infra. Precedence,
  retention, capability, normalization, initial state, and ordering are visible in the handler (or an
  entity method it calls), never hidden in a store.

- **The eight audited leaks move** (each: strip infra to raw, lift the rule into the app handler, keep
  the existing bus-level behavior test green): settings scope precedence + merge, theme
  prebuilt-immutable guard, notification retention, the daemon `kind != Terminal` capability guard
  (with `kind` removed from `SpawnRequest`), surface initial status + live-first ordering, project
  name normalization, keybinding resolve/merge/shadowing precedence, and session pinned-float ordering.

- **What stays raw:** column↔field mapping that encodes an entity field into its storage form (e.g.
  session status ↔ `archived_at`); *deterministic lexical* list sorts (by key/id) — stable query
  output, distinct from the *semantic precedence* sorts (live-first, pinned-float) that do move; the
  `get_active` pointer-then-load and `profile.duplicate` field copies (raw compositions, no rule
  checked); the migration seeds of the `Default` workspace and `Unfiled` project, whose canonical ids
  are constants owned by `entities`; and the duplicate-proxy guard in the daemon, which protects
  infra's own `proxies` map (raw resource integrity, not a domain rule). The surface_repo create
  id-minting and project create initial-state defaults are owned by the `client-assigned-create-ids`
  change, not duplicated here.

## What this amends in ADR-0036

ADR-0036 stands except for two clauses. (1) The layer-responsibility clause: where it allowed
repositories and the runtime to carry domain decisions alongside `Row -> Entity` mapping, this ADR
restricts them to raw operations and moves every domain rule to `app/`. (2) The `SurfaceRuntime` port:
where ADR-0036 named the runtime as an `Arc<dyn SurfaceRuntime>` abstraction in `Ctx`, this ADR removes
the trait in favour of a concrete `DaemonPtyApi` reached by static enum dispatch — the port indirected
over a single I/O target, so it earned its removal under the same de-abstraction rationale (and
re-aligns with ADR-0035's enum-dispatch choice). The sqlx-runtime choice, per-entity repositories, the
entity/infra/app/shared structure, and `Row -> Entity` column mapping all stand.

## Consequences

- Domain rules are read in one place — the app handler — so behavior is auditable without tracing
  through a store. infra is mechanically swappable (it makes no decisions).
- The `entities-app-or-infra-only` / `infra-only-in-app` rules can flip to `error` once the moves land,
  making a regression a build failure.
- Behavior is preserved; the bus-level tests are the safety net. One refinement: an unsupported
  surface kind is now rejected up front with an app validation error, writing no throwaway row.
- A small amount of glue shifts from infra into handlers; this is the intended cost of putting the
  rule where it belongs.
