## 1. Chrome semantics and keyboard operation

- [ ] 1.1 Audit every interactive chrome control; add one unit/component test for each ARIA, menu-semantics, nested-action, focus, tree-navigation, dialog, and menu scenario plus desktop end-to-end coverage for the cross-component keyboard journeys; then fix controls through one-scenario red-green cycles

## 2. Rendered contrast

- [ ] 2.1 Add one deterministic unit test for each contrast scenario plus rendered desktop coverage; fix failing component token usage without changing frozen token values; record current light/dark measurements in the design system

## 3. Completion gate

- [ ] 3.1 Update the completed accessibility roadmap items, run OpenSpec completeness verification and fix every finding, then run `bun run verify` and `bun run e2e` and fix every failure
