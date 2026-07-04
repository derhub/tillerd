import type { Browser } from "webdriverio";

// Throwaway capture spec for the ux-ui-overhaul visual pass. Run standalone
// (never part of run.sh's suite groups); deleted after captures land.
import { afterAll, beforeAll, expect, test } from "bun:test";

import {
  createProject,
  launchReadyApp,
  openTerminal,
  openView,
  splitPanel,
  uniqueName,
} from "./helpers";

const OUT = process.env.SHOT_DIR ?? "/tmp/shots";
let b: Browser;

async function shot(name: string): Promise<void> {
  await b.pause(800);
  await b.saveScreenshot(`${OUT}/${name}.png`);
}

beforeAll(async () => {
  b = await launchReadyApp();
}, 120_000);

afterAll(async () => {
  await b?.deleteSession().catch(() => {});
});

test("capture app states", async () => {
  await shot("01-zero-state");

  const project = uniqueName("shotproj");
  await createProject(b, project);
  await openTerminal(b);
  await shot("02-session-terminal");

  await splitPanel(b, "right");
  await shot("03-empty-panel-picker");

  const row = await b.$(`span*=${project}`);
  await b.execute((el) => {
    el?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, clientX: 120, clientY: 200 }));
  }, row as never);
  await shot("04-project-context-menu");
  await b.execute(() =>
    document.body.dispatchEvent(new MouseEvent("mousedown", { bubbles: true })),
  );

  await openView(b, "Search");
  await shot("05-search-view");
  await openView(b, "Commands");
  await shot("06-commands-view");
  await openView(b, "Templates");
  await shot("07-templates-view");
  await openView(b, "Sessions");

  await b.execute(() => {
    const bell = document.querySelector<HTMLElement>('[aria-label^="Notifications"]');
    bell?.click();
  });
  await shot("08-bottom-panel-notifications");
  const logsTab = await b.$("button*=Logs");
  if (await logsTab.isExisting()) {
    await logsTab.click();
    await shot("09-bottom-panel-logs");
  }
  await b.execute(() => {
    const t = document.querySelector<HTMLElement>('[aria-label="Toggle bottom panel"]');
    t?.click();
  });

  await b.execute(() => {
    window.history.pushState({}, "", "/settings");
    window.dispatchEvent(new Event("popstate"));
  });
  await shot("10-settings-appearance");
  const kb = await b.$("button*=Keybindings");
  if (await kb.isExisting()) {
    await kb.click();
    await shot("11-settings-keybindings");
  }
  const prof = await b.$("button*=Profiles");
  if (await prof.isExisting()) {
    await prof.click();
    await shot("12-settings-profiles");
  }

  await b.execute(() => window.dispatchEvent(new CustomEvent("command-center:open")));
  await shot("13-command-palette");
  await b.keys(["Escape"]);

  await b.execute(() => {
    window.history.pushState({}, "", "/");
    window.dispatchEvent(new Event("popstate"));
  });
  await b.execute(() => document.documentElement.classList.remove("dark"));
  await shot("14-light-home");
  await b.execute(() => document.documentElement.classList.add("dark"));

  expect(true).toBe(true);
}, 300_000);
