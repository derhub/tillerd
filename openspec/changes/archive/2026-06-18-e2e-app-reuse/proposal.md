# Reuse one desktop app across the whole e2e scenario suite

## Why

Every `test()` in the desktop e2e suite calls `launchReadyApp()`, which opens a fresh
tauri-webdriver session and cold-boots the desktop app, blocking until the embedded
orchestrator reports `services: ready`. Under the CI software-GL `xvfb` runner each
cold-boot costs ~30s, and the suite performs ~27 boots per run — roughly 14 minutes of
pure boot, dwarfing the assertions themselves.

The e2e suite's job is to confirm each spec scenario still holds through the real UI — that
"create a new project" and the like work without error. That confirmation does not need a
fresh OS-level app per scenario; it needs one running app and a clean baseline before each
scenario. `bun test` already runs the whole suite sequentially in one process, so a single
app instance can serve every scenario test.

## What Changes

- Launch the desktop app **once per run** via a `--preload` global-setup script and reuse
  that one instance for every scenario test; tear it down once when the run ends.
- Reset the shared app to a known baseline before each test (dismiss any overlay, navigate
  home) so scenario tests stay order-independent.
- Fold the dev boot-to-ready check into the shared app's own launch — the shared app reaching
  `services: ready` is the dev boot assertion; no separate dev-mode boot launch.
- Keep `resume.test.ts` self-launching (it asserts a real restart) and run it as its own
  `bun test` invocation, outside the shared-app preload.
- Keep the bundled-boot invocation, now reusing the same preload mechanism against the
  release binary.

Net boots per run: ~3 (one shared dev app + one resume + one bundled boot), down from ~27.

Out of scope: changing what any scenario asserts, daemon/runtime-dir provisioning, the
bundled-vs-dev coverage split, or CI topology beyond the run.sh invocation structure.

## Capabilities

- **dev-verification** (modified): add a requirement that the e2e scenario suite shares one
  app instance for the whole run and stays order-independent through a between-tests reset,
  without weakening the existing self-provisioning / isolated-runtime / no-orphan guarantees,
  and while preserving the dev+bundled boot and resume-after-restart coverage.

## Risks

- **Cross-test state leakage** — one app for the whole run means residual state (overlays,
  routes, projects, log rows) can bleed between tests. Mitigation: an asserted reset baseline
  before each test; tests already target entities by unique name under the shared `TILLERD_DIR`.
- **Whole-suite crash cascade** — if the shared app dies mid-run, every remaining scenario
  test fails. Mitigation: `--bail` already stops at the first failure; the run.sh `EXIT` trap
  reaps the app and services; lifecycle specs (boot/resume) run as separate invocations so a
  scenario crash cannot mask them.
- **Name collisions under back-to-back tests** — without a ~30s boot between tests, two
  `Date.now()` names can land in the same millisecond. Mitigation: a per-run monotonic counter
  in the unique-name helper.
- **Global-setup mechanism** — relies on bun `--preload` run-scoped `beforeAll`/`afterAll`
  (documented) and a single-process run (run.sh already guarantees it). Validated first.
