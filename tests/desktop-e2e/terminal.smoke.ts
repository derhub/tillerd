import { remote } from "webdriverio";

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

async function dumpDom(): Promise<void> {
  try {
    const dom = await browser.execute(() => ({
      url: location.href,
      body: (document.body?.innerText ?? "").slice(0, 400),
      buttons: Array.from(document.querySelectorAll("button")).map((b) => b.textContent?.trim()),
    }));
    console.error("DOM on failure:", JSON.stringify(dom));
  } catch {
    // best-effort diagnostics only
  }
}

try {
  // "New session" only navigates to the terminal once the orchestrator is ready; before that it
  // takes the (dead) web path. Wait for ready first.
  await browser.waitUntil(
    async () => (await browser.$("body").getText()).includes("orchestrator: ready"),
    { timeout: 45_000, timeoutMsg: "orchestrator did not reach ready" },
  );

  // The "New session" control is an icon button (no text), named by its title attribute.
  const newSession = await browser.$('button[title*="New session"]');
  await newSession.waitForExist({ timeout: 10_000 });
  await newSession.click();
  await browser.waitUntil(async () => (await browser.getUrl()).includes("/session/"), {
    timeout: 10_000,
    timeoutMsg: "clicking New session did not navigate to a session route",
  });

  // The pane creates a terminal surface; the daemon's shell streams through the orchestrator and
  // paints xterm. Non-empty rendered text proves the end-to-end path.
  const term = await browser.$(".xterm");
  await term.waitForExist({ timeout: 20_000 });
  await browser.waitUntil(async () => (await term.getText()).trim().length > 0, {
    timeout: 20_000,
    timeoutMsg: "terminal did not render streamed output",
  });

  const painted = (await term.getText()).trim();
  console.log(`PASS: terminal rendered ${painted.length} chars of streamed output`);
} catch (err) {
  await dumpDom();
  throw err;
} finally {
  await browser.deleteSession();
}
