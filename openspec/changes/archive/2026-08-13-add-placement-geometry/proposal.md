## Why

Panel dividers resize only in the mounted view; the panel tree does not own child sizes, so reloads lose the user's geometry and nested splits cannot restore their independent proportions. Roadmap 0.1.1 completes the frozen placement model with additive, durable geometry.

## What Changes

- Store normalized child-size ratios on every split group and restore each nested group's sizes independently.
- Persist divider drags and equal-split resets through the existing per-session layout write path.
- Keep placement identifiers, surface ownership, session storage columns, and transport contracts unchanged.
- **BREAKING**: replace the unversioned panel-tree blob with a versioned layout envelope. Existing unversioned layouts fail with an incompatible-layout error; no compatibility fallback or migration is provided.
- Existing development geometry remains unreadable until the user discards the pre-change development data.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui-panel-model`: Split groups own validated child-size ratios, including nested groups.
- `ui-panel-compound`: Resizable groups render persisted ratios and write drag/reset results back to the panel tree.
- `layout-persistence`: Resize mutations persist in the versioned per-session layout blob; incompatible unversioned blobs report an error instead of being inferred, migrated, or reset.

## Impact

- UI model and hooks: `apps/ui/app/lib/panelTree.ts`, `apps/ui/app/lib/usePanelTree.ts`.
- Panel rendering: `apps/ui/app/components/shell/PanelGroup.tsx`, `apps/ui/app/components/shell/PanelTree.tsx`.
- Unit and desktop E2E coverage for nested resize, reset, reload, and clean-cutover behavior.
- No new dependency, database migration, orchestrator command, wire-field, or design-token change.
