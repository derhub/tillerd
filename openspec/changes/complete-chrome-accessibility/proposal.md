## Why

The working-app milestone still has incomplete accessibility gates: nested chrome actions do not consistently expose names, tooltips, keyboard focus, or visible focus states, and rendered color pairings lack a complete WCAG AA audit. Closing these gates completes the milestone without changing application architecture.

## What Changes

- Audit all interactive desktop chrome and fix missing semantic elements, accessible names, state attributes, icon-only tooltips, and visible focus treatment.
- Complete keyboard operation for sidebar actions, panel actions, dialogs, and menus using standard Tab, Shift+Tab, arrow, Enter, and Escape behavior.
- Measure rendered foreground/background pairings in light and dark themes, adjust tokens or usage where needed, and record the verified ratios.
- Add component tests for each accessibility scenario and desktop end-to-end coverage for cross-component keyboard flow.
- Mark the completed accessibility roadmap items after verification.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui-accessibility`: Complete ARIA, tooltip, focus, keyboard-navigation, and rendered contrast requirements across all interactive chrome.

## Impact

- Affected code: desktop renderer components, shared chrome primitives, theme tokens, component tests, and desktop end-to-end tests.
- Affected documentation: design token contrast record and roadmap status.
- Dependencies: none.
- Contracts: no service, wire, data-model, runtime-layout, extension, or panel-placement contract changes.
