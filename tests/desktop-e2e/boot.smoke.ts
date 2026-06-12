import { remote } from "webdriverio";

// Boot canary: the embedded orchestrator (gate + daemon supervised) must reach `ready`. Runs
// first so a backend-boot regression fails fast and unambiguously, before the heavier UI flows.

const application = process.env.TILLERD_DESKTOP_BIN;
if (!application) {
  throw new Error("set TILLERD_DESKTOP_BIN to the built desktop binary");
}

const browser = await remote({
  hostname: "127.0.0.1",
  port: 4444,
  path: "/",
  capabilities: { "tauri:options": { application } } as Record<string, unknown>,
});

try {
  await browser.waitUntil(
    async () => (await browser.$("body").getText()).includes("orchestrator: ready"),
    { timeout: 45_000, timeoutMsg: "orchestrator did not reach ready" },
  );
  console.log("PASS: orchestrator reached ready");
} finally {
  await browser.deleteSession();
}
