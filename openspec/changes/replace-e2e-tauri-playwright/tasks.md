# Tasks

## 1. Spike (gates everything after)

- [ ] 1.1 Prove the toolchain: add `@srsholmes/tauri-playwright` (exact pin) + `@playwright/test` to a spike e2e workspace; confirm the package resolves under bun workspaces (upstream exports-map issue); add the plugin behind an `e2e-testing` cargo feature with an e2e-only Tauri config overlay (`withGlobalTauri`, `playwright:default` capability with panel-detach window labels in scope); boot one smoke spec in `tauri` mode against the real binary on macOS and Linux. Resolves both design Open Questions (config-merge capability handling, navigation workaround need). If bun-workspace resolution is unfixable without upstream changes, stop and report.

## 2. Harness

- [ ] 2.1 Port the harness contract to Playwright primitives: `scenarios` project with shared-app global setup/fixtures (one boot, isolated `TILLERD_DIR`, per-test baseline reset, teardown reaps all processes) and `lifecycle` project for own-launch specs (resume, deep-route reload, view-pointers restart, foundation-integration, `E2E_BUNDLED` boot check); helpers (`launchReadyApp`, `resetToHome`, `uniqueName`, `stubPrompt`, surface/log utilities) become fixtures — specs never import the tool directly; thin build pre-step in the package `e2e` script replaces run.sh build/preflight. Traces to dev-verification: self-provisioning, shared-instance, lifecycle-own-launch, no-orphan requirements.

## 3. Spec port

- [ ] 3.1 Port all 22 specs webdriverio → Playwright, spec-scenario → test 1:1 names preserved; retry each WebDriver-era workaround through the new driver first, keep only where the webview limitation persists; shared-app group first, lifecycle group last, full suite green between groups plus one shuffled-order run.

## 4. Cutover

- [ ] 4.1 Flip CI e2e job (ubuntu+macos matrix) to the new runner (drop `cargo install tauri-webdriver`, add Playwright install, keep xvfb + apt deps + `E2E_BUNDLED`, keep 30-min timeout); remove run.sh, webdriverio deps, `webdriver` cargo feature, and the old plugin; verify release build compiles no automation bridge and opens no endpoint (ADDED spec requirement); update project docs that referenced the old driver.

## 5. Verify gate

- [ ] 5.1 Run `/opsx:verify` and fix all issues; then `bun run verify` and fix all issues; then `bun run e2e` (both local + confirm CI green on both OSes) and fix all issues.
