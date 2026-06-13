import { afterEach, expect, test } from "bun:test";
import { type Browser, launchReadyApp } from "./helpers";

// Boot canary: the embedded orchestrator (gate + daemon supervised) must reach `ready`. Runs first
// so a backend-boot regression fails fast and unambiguously, before the heavier UI flows.

let browser: Browser | undefined;
afterEach(async () => {
  await browser?.deleteSession();
  browser = undefined;
});

test("boots to a ready shell", async () => {
  browser = await launchReadyApp();
  expect(await browser.$("body").getText()).toContain("services: ready");
}, 120_000);
