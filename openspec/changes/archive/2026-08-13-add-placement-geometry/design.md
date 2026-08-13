## Context

Split groups currently persist direction and children but not child sizes. The installed resize primitive therefore owns transient proportions and reload reconstructs each panel from a hard-coded default. The existing `session_layout_set` command already persists the full panel tree in `session.layout_json`; no backend or transport change is needed.

ADR-0030 freezes placement ownership: panel geometry binds leaves to orchestrator-minted placement identifiers, while the launch spec owns surfaces. This change extends that stable boundary only with geometry. Placement identifiers, bindings, surface lifecycle, and the launch spec remain unchanged. The user explicitly approved a clean cutover with no legacy-layout fallback.

## Goals / Non-Goals

**Goals:**

- Persist normalized child-size percentages per split group.
- Restore nested groups with independent proportions.
- Persist completed pointer and keyboard resizes plus equal-split resets.
- Reject incompatible or malformed stored layouts instead of inventing geometry.

**Non-Goals:**

- Changing placement identifiers, surface ownership, launch specs, database columns, or transport commands.
- Migrating or inferring geometry for existing unversioned layout blobs.
- Adding display modes, tabs, minimum-size settings, snap points, or new design tokens.
- Adding a package or crate.

## Decisions

### Store sizes on each split group

Each group stores a `sizes: number[]` beside `children`; indices align with child order. The model normalizes finite non-negative values to sum to 100, rejects missing or wrong-length arrays and all-zero totals, assigns equal values to a new split, preserves surviving shares when a child closes, and updates one group by stable group identifier. Nested callbacks cannot alter ancestor or sibling geometry.

This keeps geometry beside the tree node it describes and naturally supports nested groups. A separate size map was rejected because it duplicates tree identity and needs orphan cleanup after close/collapse.

### Version the serialized layout and use a clean cutover

`serializeLayout` writes `{ "version": 1, "root": tree }`. `deserializeLayout` accepts only version 1 and fully validates the recursive tree, including size cardinality and values, unique non-empty node and placement identifiers, active-tab membership, and the existing leaf invariants. Existing unversioned blobs and unknown versions fail deserialization. A null layout still creates the single-empty-leaf default; an incompatible non-null layout surfaces an error instead of silently replacing bindings.

Inferring equal sizes for old groups was rejected by explicit user direction. This is a pre-1.0 view-state cutover: users with pre-change development data must discard it before opening those sessions. The implementation does not parse, migrate, reconcile, or overwrite incompatible geometry.

`deserializeLayout` throws a typed `LayoutFormatError`. The layout loader distinguishes null from incompatible data and `usePanelTree` exposes `layoutError` with its otherwise inert tree state. `PanelContent` renders a blocking `role="alert"` state instead of `PanelTree` when that error is present. The shared sidebar remains mounted, so its existing delete-session flow lets the user discard the incompatible development session; this change adds no reset or migration path.

### Use the installed resizable primitive as the persistence boundary

Give every panel and group its stable tree identifier. Prefix panel identifiers only at the primitive boundary so numeric tree identifiers cannot be reordered as JavaScript array-index keys. Build `defaultLayout` from those library-facing child identifiers to stored percentages and pass it to `ResizablePanelGroup`. Handle `onLayoutChanged`; compare the returned identifier map with the owning group's current stored sizes and persist only when they differ. This ignores the primitive's initial completed callback because it reports the same default layout, while completed pointer or keyboard resizes report changed values. Convert the identifier map back to child order before updating the owning group.

This reuses the installed dependency's documented completed-resize callback rather than adding pointer listeners or storage. Persisting `onLayoutChange` was rejected because it writes on every pointer move. The installed v4.11.2 callback has no interaction metadata; equality against the controlled model is the explicit programmatic/default-layout guard.

### Reset explicitly in both the primitive and model

The separator disables the primitive's built-in double-click behavior. Its own double-click handler computes equal shares, calls the group imperative API to update the mounted panels, and directly updates the owning tree group for persistence. It MUST NOT rely on `onLayoutChanged`: imperative `setLayout` may trigger the same completed callback, and the callback cannot distinguish its origin. Treating the stored layout as the primitive's default was also rejected because double-click would restore the old user sizes instead of equal shares.

### Test at model, component, and desktop boundaries

Each spec scenario gets a deterministic unit test. Pure model tests cover creation, nested isolation, normalization, validation, version restore, and clean cutover. Component tests cover stored layout wiring, completed user resize reporting, equal reset reporting, and the blocking incompatible-layout state. Desktop E2E journeys cover nested resize and reload plus incompatible-layout reporting through the real session layout command; these behaviors cross the webview, component, IPC, and database boundaries.

## Risks / Trade-offs

- [Existing saved layouts become unreadable] -> Explicitly approved clean cutover; surface an incompatible-layout error and require pre-change development data to be discarded rather than silently hiding bound surfaces.
- [Floating-point drift] -> Normalize at the model boundary and compare with a small tolerance in tests.
- [Library callback on initial render] -> Ignore a completed layout that equals the owning group's stored sizes; component coverage proves mount is inert and a changed completed layout persists.
- [Nested callback updates the wrong group] -> Every callback closes over its own stable group identifier; unit coverage proves sibling and ancestor isolation.
- [E2E pointer actions are unreliable in the WebKit driver] -> Exercise the primitive's accessible separator with two directional key presses, then assert proportional geometry with tolerance.
