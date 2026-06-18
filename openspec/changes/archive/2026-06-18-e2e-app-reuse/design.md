## Context

`launchReadyApp()` opens a tauri-webdriver session and cold-boots the desktop app, blocking
on `services: ready` (45s ceiling). Each `test()` calls it; `afterEach` calls
`deleteSession()`. Under the CI software-GL `xvfb` runner a boot costs ~30s, and the suite
runs ~27 of them.

`run.sh` establishes the facts this design relies on:
- `bun test` runs every `*.test.ts` **sequentially in one process** — so a module-level
  singleton `browser` can be shared across all files.
- tauri-webdriver launches an app **per WebDriver session** — so reuse means creating one
  session once and sharing it.
- One isolated `TILLERD_DIR` is shared across the whole run already; bundled-boot runs as a
  **separate** `bun test` invocation; an `EXIT` trap reaps the app, services, and webdriver.

The suite is also already built for a shared `TILLERD_DIR`: tests create uniquely-named
projects (`Project ${Date.now()}`), target them by name (never by position), and tolerate
other tests' residual entities. Reuse therefore needs no backend wipe.

bun supports run-scoped `beforeAll`/`afterAll`/`beforeEach` defined in a `--preload` script
(Bun "Global Setup and Teardown") — the mechanism for launching once and resetting per test.

## Resolved decisions (readiness gate)

- **Placement / deps:** new files (`setup.ts`, `shared-app.ts`) in `tests/desktop-e2e/`; no new crate, package, or dependency — reuses webdriverio + bun:test.
- **Mechanism:** bun `--preload` run-scoped hooks + module singleton; verified against Bun docs and the single-process run guarantee.
- **Frozen seams:** none touched (test harness only); the `services: ready` ready-signal is preserved, not changed.
- **Verify + env:** `bun run verify` + e2e via `run.sh`; macOS host (no GTK/xvfb); preflight installs deps and confirms `tauri-webdriver` present.
- **Build artifact:** `tauri build` rewrites `tauri.conf.json` (drops `maximized`) — a build artifact, `git checkout` it; never commit. New untracked files may be swept by the ASCII Stop-hook `style:` commit — split before pushing.
- **Skills:** `.ts`/`.sh` → my-writing-style + my-code-style; no TDD (harness refactor covered by existing e2e specs).

## Goals / Non-Goals

**Goals:**

- One app launch for the whole scenario run, reused by every scenario test, reaped once.
- A light, asserted between-tests reset that keeps scenario tests order-independent.
- Fold dev boot-to-ready into the shared launch; no change to what any scenario asserts.

**Non-Goals:**

- Sharing the app with `resume.test.ts` (real restart) or bundled boot (release binary) —
  both are lifecycle tests that need their own launches.
- Deleting projects/sessions or resetting the orchestrator store between tests.
- Silencing a11y/DRI3 warnings or tuning boot time (separate, later concerns).

## Decisions

**Global shared app via preload.** Add `tests/desktop-e2e/setup.ts` (a `--preload` script)
and `tests/desktop-e2e/shared-app.ts` (the singleton holder). `setup.ts`:
- `beforeAll`: launch the app (the existing `remote()` + wait-for-`services: ready` logic,
  moved into `shared-app.ts`'s `launchSharedApp()`), store the `browser` in the module.
- `afterAll`: `deleteSession()` and clear the holder.
- `beforeEach`: `resetToHome(getApp())`.

Scenario specs and `boot.test.ts` obtain the instance with `getApp()`; they declare no launch
and no per-test teardown.

**Asserted reset, in beforeEach.** `resetToHome(browser)` in `helpers.ts`:
1. `browser.keys(['Escape'])` (the cmdk palette input is autofocused when open and
   `CommandCenter` closes on Escape; context menus, dialogs, and the inline-rename input also
   close on Escape), then a `mousedown` on `document.body` as belt-and-suspenders.
2. Client-side navigate home (`history.pushState('/')` + `popstate`, mirroring `openLogViewer`).
3. **Assert** the baseline: `waitUntil` no `[role=menu]`, `[role=alertdialog]`,
   `[data-testid="inline-rename-input"]`, or open palette exists, AND the "New project"
   affordance is present. A botched dismiss fails IN the reset (clear cause).

Running it in `beforeEach` means each test ENTERS clean regardless of how the prior ended; a
failing reset attributes to the about-to-run test. The first `beforeEach` runs against the
fresh launch — a near no-op.

**Boot folds in; resume and bundled split out.** `boot.test.ts` asserts `getApp()` shows the
ready shell — that assertion, satisfied by the shared launch, is the dev boot-to-ready check.
`run.sh` runs three invocations:
- Main (dev): `bun test --bail --preload setup.ts <all *.test.ts except resume.test.ts>`.
- Resume: `bun test --bail tests/desktop-e2e/resume.test.ts` (no preload; self-launches, real
  restart). Kept out of the preloaded run so it never collides with the shared app.
- Bundled (when `E2E_BUNDLED`): `bun test --bail --preload setup.ts boot.test.ts` with
  `BUNDLED_BIN` — the same preload launches the release binary once and `boot.test.ts` asserts
  ready.

Excluding resume from the main run is a shell glob (`ls *.test.ts | grep -v resume`).

**Unique names under back-to-back execution.** Without a ~30s boot between tests, two
`Date.now()` calls can collide. Add a per-run monotonic counter to the name helper
(`uniqueName(prefix)` → `${prefix} ${Date.now()}-${n++}`); switch existing
`Project ${Date.now()}` call sites to it.

**Rollout order.** Land the harness (setup.ts, shared-app.ts, resetToHome, run.sh wiring)
plus `boot.test.ts` first and prove the shared app boots and reaches ready. Then migrate
scenario specs off `launchReadyApp()` onto `getApp()`, simplest first (single-test specs:
delete their `afterEach`/launch), then the multi-test specs, running the suite green after each.

## Risks / Trade-offs

- **Cross-test leakage** — mitigated by the asserted `beforeEach` reset (fails fast and
  visibly if an overlay survives). Order-independence verified by running the full suite twice
  (catches cross-run accumulation in the shared `TILLERD_DIR`) plus a one-time manual reorder
  spot-check during rollout — no shuffle harness built.
- **Whole-suite crash cascade** — if the shared app dies, remaining scenario tests fail.
  Accepted: `--bail` stops at the first failure, the `EXIT` trap reaps, and lifecycle specs run
  as separate invocations so they are never masked. No relaunch-on-crash (YAGNI).
  Granularity is a deliberate choice: the documented Tauri/WebdriverIO norm is per-spec-file
  session reuse (~16 boots, crash blasts one file), which every official example follows.
  Whole-run reuse (~3 boots) is an extension this design takes for the larger win, viable
  because `bun test` is single-process; the concentrated crash risk is the trade, mitigated as
  above. Reuse-over-relaunch itself is the established best practice — per-test relaunch (the
  current suite) is the known-slow anti-pattern.
- **Name collision** — addressed by the per-run counter.
- **`panel-detach` windows** — non-issue: tauri-webdriver drives a single webview and cannot
  spawn child windows, so those tests assert parent-side DOM only; residual detach state
  attaches to each test's uniquely-named project.
- **Preload/process assumptions** — validated in the first task before migrating specs: the
  preload launches once, `getApp()` returns it across files, and the run stays single-process.
