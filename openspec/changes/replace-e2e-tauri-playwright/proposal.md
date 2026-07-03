## Why

The desktop e2e harness (tauri-webdriver + bun test + a 117-line run.sh) carries hand-rolled orchestration and WebDriver-era workarounds (raw `execute` with no Promise returns, pushState navigation hacks, synthetic-event dispatch) that make specs slow to write and brittle to maintain. Replacing it with `@srsholmes/tauri-playwright` moves the suite onto the Playwright test runner with a maintained Tauri v2 integration, following the tool's documented configuration instead of bespoke harness glue.

Decision context: alternatives were evaluated (keep-and-optimize; official `@wdio/tauri-service`) and the tradeoffs of tauri-playwright are accepted — young single-maintainer project, Windows `tauri` mode currently broken (upstream issue #12; Windows is not on the committed roadmap), and on macOS/Linux the driver is an eval bridge into the real webview rather than Playwright's own browser engine.

## What Changes

- Add `tauri-plugin-playwright` to the desktop app behind an opt-in `e2e-testing` cargo feature; production builds never compile it. **BREAKING** for the e2e workflow only: the `webdriver` feature and embedded `tauri-plugin-webdriver` are removed.
- Set `withGlobalTauri: true` and add the `playwright:default` permission to `src-tauri/capabilities`, with the `windows` scope widened to cover detached panel windows (avoids the documented silent 30s eval hang).
- Port all 22 specs (~1945 LOC) plus helpers from the webdriverio API to the Playwright API, running in `tauri` mode (real binary, real IPC) as the primary mode.
- Replace `tests/desktop-e2e/run.sh` orchestration (build, preflight boot, driver startup, two-pass spec grouping, process reaping) with Playwright config/projects and global setup/teardown per the tool's documentation, preserving the shared-app-instance model, isolated `TILLERD_DIR`, and bundled-build boot check.
- Rewire the CI e2e job (ubuntu + macos matrix): drop `cargo install tauri-webdriver`, install Playwright tooling instead.
- Remove the `tauri-webdriver`/`tauri-plugin-webdriver` integration and webdriverio dependencies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `dev-verification`: driver-agnostic wording for the shared-app teardown requirement (webdriver session → automation session), plus a new requirement that e2e automation instrumentation is feature-gated and absent from production builds. All other requirements (self-provisioning, one-command battery, CI coverage, shared instance, order independence) are unchanged and must hold under the new runner.

## Impact

- `tests/desktop-e2e/**` — full rewrite of harness and specs (runner, helpers, setup, orchestration).
- `apps/desktop/src-tauri` (Cargo.toml features, plugin init, `tauri.conf.json`, `capabilities/*.json`).
- `.github/workflows/ci.yml` e2e job (lines 82-119): toolchain install, invocation, env.
- Root `package.json` / `turbo.json` e2e task chain; `tests/desktop-e2e/package.json` dependencies (`@srsholmes/tauri-playwright@0.4.1`, `@playwright/test`; webdriverio removed).
- Risk to verify early: upstream issue #4 (package exports resolution failure in pnpm monorepos) — confirm the package resolves under bun workspaces before porting specs.
- Docs/memory that reference tauri-webdriver gotchas become stale after landing (project memory `testing.md`, ROADMAP.md:105-111 history note).
