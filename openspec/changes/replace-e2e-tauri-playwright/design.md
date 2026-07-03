## Context

The desktop e2e suite (22 specs, ~1945 LOC, `tests/desktop-e2e/`) drives the real Tauri binary through tauri-webdriver (W3C client + embedded `tauri-plugin-webdriver` behind a `webdriver` cargo feature). Orchestration lives in `run.sh`: build services/UI/debug binary, preflight boot check, start the driver, run bun test in two passes (shared-app scenarios, then own-launch lifecycle specs), reap processes. CI runs the suite on an ubuntu + macos matrix with a 30-minute timeout and installs the driver via `cargo install` on every run.

The replacement, `@srsholmes/tauri-playwright@0.4.1`, integrates the Playwright test runner with Tauri v2. On macOS/Linux its `tauri` mode spawns the real app and drives the real webview through an in-app plugin: a local socket receives commands, the plugin evaluates generated scripts in the webview, and results return over real IPC. Playwright's own browser engine is not involved in this mode; locator and auto-wait semantics are reimplemented by the plugin. The eval bridge awaits async expressions and returns their values — a capability the WebDriver `execute` path lacked.

## Goals / Non-Goals

**Goals:**

- Whole suite on the Playwright test runner, `tauri` mode (real binary, real services, real IPC), per the tool's documented configuration — no hand-rolled harness glue beyond what the tool requires.
- Preserve every `dev-verification` requirement: self-provisioning, isolated runtime dir, no orphans, one-command battery, shared app instance for scenarios, own launches for lifecycle specs, bundled boot check, CI on both OSes.
- Automation instrumentation feature-gated out of production builds.

**Non-Goals:**

- Windows support (upstream `tauri` mode is broken there; Windows is not on the committed roadmap).
- `browser` mode (mocked IPC) — unit/component tests already cover UI-only behavior; adopting it would duplicate that layer with a weaker contract.
- Visual regression / screenshot assertions (default screenshot is a DOM-serialization rasterization; unreliable as a pixel contract).
- Changing what the suite covers — this is a harness port, scenario for scenario.

## Decisions

- **tauri-playwright over keep-and-optimize or the first-party WebDriver service.** User decision after evidence review: Playwright runner ergonomics (fixtures, projects, trace/report tooling, async eval) valued over the alternatives; accepted costs are project youth, single maintainer, and no Windows. Alternatives considered: keep tauri-webdriver and fix speed (lowest risk, keeps WebDriver ergonomics); official WebdriverIO service (first-party, all-OS, same API as current specs, but stays on WebDriver semantics).
- **`tauri` mode only, spike before port.** First task proves the package resolves under bun workspaces (upstream has an exports-map resolution issue reported in pnpm monorepos) and boots one smoke spec against the real binary on macOS and Linux. The 22-spec port starts only after the spike is green on CI. Rollback at any point = keep the existing harness; it remains intact on main until the port lands.
- **Feature gate mirrors the current pattern.** `e2e-testing` cargo feature gates the plugin dependency and its `init()` registration, replacing the `webdriver` feature one-for-one. Release builds compile neither bridge.
- **E2E-only Tauri config overlay.** `withGlobalTauri: true` and the `playwright:default` capability (with `windows` scope widened to detached panel windows — undersized scope hangs evals silently for 30s) live in an e2e overlay config merged via the Tauri CLI `--config` flag at e2e build time. Production `tauri.conf.json` and capabilities stay untouched; the ACL never references the plugin in non-e2e builds.
- **Playwright projects replace the run.sh two-pass split.** A `scenarios` project uses worker-scoped fixtures/global setup for the shared app instance (one boot, per-test baseline reset, teardown reaps everything); a `lifecycle` project runs resume-after-restart, deep-route reload, view-pointers restart, foundation-integration, and the bundled boot check as own launches. Build orchestration (turbo-built services/UI/binary) stays a thin pre-step in the package `e2e` script; preflight boot check folds into global setup.
- **Playwright runner for this package only.** The e2e workspace departs from the repo's bun-test default because the runner is the point of the tool; every other package stays on `bun test`. Suite keeps spec-scenario → test 1:1 naming so `dev-verification` traceability survives the port.
- **Helpers become fixtures, workarounds re-evaluated per spec.** `launchReadyApp`, `resetToHome`, `uniqueName`, `stubPrompt`, surface/log helpers port to Playwright fixtures. Each WebDriver-era workaround (pushState navigation, synthetic `dblclick`/`contextmenu` dispatch, select-change dispatch) is retried through the new driver first and kept only where the eval bridge shares the underlying webview limitation.
- **Pin the dependency.** Exact-version pin of `@srsholmes/tauri-playwright`; upgrades are deliberate, reviewed bumps (young project, single maintainer, API surface still moving).

## Risks / Trade-offs

- [Upstream abandonment or breaking churn — 3-month-old, one maintainer] → exact pin; harness boundary kept thin (fixtures wrap the tool, specs never import it directly) so a future driver swap re-touches fixtures, not 22 specs; the removed harness remains recoverable from git history.
- [Package fails to resolve under bun workspaces (upstream exports-map issue)] → spike task gates the port; if unresolvable without upstream changes, stop and report before any spec is touched.
- [Capability `windows` scope too narrow → silent 30s hangs in multi-window specs] → overlay capability written against the panel-detach window labels from day one; hang symptoms documented in the harness readme section.
- [Eval bridge shares webview event limitations — React synthetic events, xterm keyboard may still not work] → per-spec re-evaluation during port; keep the existing DOM-affordance assertion style where input synthesis fails; no scenario coverage is dropped silently.
- [In-app socket server in test builds is an attack surface if it leaks into release] → cargo feature gate plus the added spec requirement (release build: no bridge compiled, no endpoint listening) verified by a release-build check in the suite.
- [Shared-app model must be rebuilt on Playwright primitives — order-independence regressions possible] → port the setup/reset contract as fixtures first, then move specs in groups with the full suite green between groups (shuffled-order run before calling the port done).
- [CI timing unknown under the new runner] → keep the 30-minute timeout initially; measure and tighten after the port stabilizes. Dropping the per-run `cargo install tauri-webdriver` is an expected win.

## Migration Plan

1. Spike lands behind a new directory (`tests/desktop-e2e-pw/` or a branch-local rename) without touching the existing suite; CI runs both only if cheap, otherwise the spike runs manually on both OSes.
2. Port specs in groups (shared-app scenarios first, lifecycle last); old suite stays the CI gate until the new suite is fully green on both OSes.
3. Flip CI to the new suite, remove `run.sh`, webdriverio deps, the `webdriver` cargo feature, and `tauri-plugin-webdriver`; rename the directory back to `tests/desktop-e2e/`.
4. Rollback at any step: revert the flip commit; the old harness is a single revert away until step 3 merges.

## Open Questions

- Does the Tauri CLI `--config` merge cover capability additions cleanly, or does the e2e build need a scripted capability copy? Resolve during the spike.
- Does the bridge's navigation API remove the pushState workaround, or does custom-scheme SPA navigation still require it? Resolve during the first scenario-group port.
