## Context

The desktop renderer already uses semantic controls, accessible component primitives, a roving-tabindex sidebar tree, and shared light/dark design tokens. Coverage is uneven in nested sidebar actions, raw icon buttons, and visible focus treatment. The design system contains contrast measurements, but the open roadmap gate requires verification against rendered component states.

The change is renderer-only. It does not change service, wire, data-model, runtime-layout, extension, or panel-placement contracts. Existing design-token values are frozen and remain unchanged.

## Goals / Non-Goals

**Goals:**

- Make every interactive chrome action semantically named, keyboard reachable, and visibly focused.
- Preserve standard keyboard behavior in composite widgets and overlays.
- Verify required rendered foreground/background pairings in both themes and record current measurements.
- Close the three remaining accessibility roadmap items with scenario-linked tests.

**Non-Goals:**

- Screen-reader support inside the terminal canvas.
- New navigation models, configurable shortcuts, or visual redesign.
- Changes to frozen token values or backend contracts.
- Mobile, web-host, or platform behavior outside the current macOS and Linux desktop targets.

## Decisions

### Audit and fix existing controls in place

Use native semantic elements and existing shared primitives. Add accessible names, tooltip composition, focus-visible classes, and keyboard visibility to the controls that lack them. Do not add a wrapper abstraction or dependency for a finite audit.

Alternative: add a new universal chrome-button component. Rejected because current controls vary by context and a new abstraction would increase migration scope without enforcing semantic correctness.

### Preserve composite-widget keyboard ownership

Keep arrow-key navigation in the sidebar tree and existing menu/dialog primitives. Nested row actions remain ordinary tab stops and stop event propagation where required, so tree navigation and direct action activation do not conflict.

Alternative: make every nested action an arrow-key descendant of the tree. Rejected because it changes the established tree interaction model and complicates row navigation.

### Fix contrast through token usage, not token values

Measure actual rendered combinations. Replace a failing component usage with an existing compliant foreground or background token. Keep decorative low-contrast separators only where they are not required to identify or operate a control, and record the measured exception.

Alternative: change palette values globally. Rejected because design-token values are a frozen 0.x contract and global changes would affect unrelated surfaces.

### Test each scenario at the narrowest observable boundary

- Unit/component: icon-only naming and tooltip behavior; nested sidebar action reachability; focus-visible treatment; sidebar key routing; overlay focus restoration; contrast calculation and required pairings.
- Desktop end-to-end: one representative sidebar-to-action keyboard journey and one dialog/menu focus round trip because these cross component and webview boundaries.
- Rendered contrast: browser-level representative states in both themes plus deterministic unit checks for color calculations and token pair thresholds.
- Full verification: `bun run verify`, which includes formatting, type checking, lint, unit tests, and the complete desktop end-to-end suite.

The existing component-test and desktop-driver harnesses cover every required layer. No new dependency or harness is needed.

## Plan Critique and Resolution

- **Critique:** "All interactive chrome" can expand into unrelated visual cleanup. **Resolution:** begin from a finite structural inventory of interactive renderer controls and permit only semantic, keyboard, focus, tooltip, and failing token-usage edits.
- **Critique:** the first task draft did not name every specification scenario, so required 1:1 unit coverage could be missed. **Resolution:** the task now requires one unit/component test per ARIA, menu-semantics, nested-action, focus, tree-navigation, dialog, menu, and contrast scenario, with end-to-end coverage only where behavior crosses components or the desktop webview.
- **Critique:** contrast remediation could accidentally change frozen design tokens. **Resolution:** token definitions are protected; remediation may only select an existing compliant token at the failing component use.
- **Critique:** rendered contrast checks alone can be flaky or incomplete. **Resolution:** deterministic color-calculation unit tests are authoritative for ratios; desktop checks prove representative computed styles and theme application.
- **Critique:** splitting implementation across writers would create conflicts in shared chrome components, tests, design documentation, and the roadmap. **Resolution:** apply serially.

## Risks / Trade-offs

- A broad audit can drift into visual cleanup. Mitigation: change only semantics, keyboard operation, focus visibility, and failing rendered token usage.
- Hover-reveal actions can remain visually hidden to keyboard users. Mitigation: every such action becomes visible under `focus-visible` or row `focus-within`.
- Automated contrast coverage cannot prove every possible runtime composition. Mitigation: enumerate representative rendered states and retain the design-system pairing table as the auditable source of truth.
- Nested controls inside tree rows can leak key events to tree navigation. Mitigation: exercise direct activation and row traversal in both component and desktop tests.
