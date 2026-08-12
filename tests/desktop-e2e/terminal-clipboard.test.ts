import { expect, test } from "bun:test";

import { createProject, openTerminal, type Browser } from "./helpers";
import { getApp } from "./shared-app";

async function writeSystemClipboard(value: string): Promise<void> {
  const command =
    process.platform === "darwin"
      ? ["pbcopy"]
      : Bun.which("wl-copy")
        ? ["wl-copy"]
        : ["xclip", "-selection", "clipboard"];
  const proc = Bun.spawn(command, { stdin: "pipe" });
  await proc.stdin.write(value);
  await proc.stdin.end();
  if ((await proc.exited) !== 0)
    throw new Error(`failed to write system clipboard with ${command[0]}`);
}

async function readSystemClipboard(): Promise<string> {
  const command =
    process.platform === "darwin"
      ? ["pbpaste"]
      : Bun.which("wl-paste")
        ? ["wl-paste"]
        : Bun.which("xclip")
          ? ["xclip", "-selection", "clipboard", "-o"]
          : [];
  if (command.length === 0) {
    throw new Error("Linux clipboard prerequisite missing: install wl-paste or xclip");
  }
  const proc = Bun.spawn(command, { stdout: "pipe" });
  const output = await new Response(proc.stdout).text();
  if ((await proc.exited) !== 0)
    throw new Error(`failed to read system clipboard with ${command[0]}`);
  return output;
}
async function runClipboardCommand(
  b: Browser,
  label: "Copy" | "Paste" | "Select all",
): Promise<void> {
  await b.execute(() => {
    const terminal = document.querySelector(".xterm");
    if (!terminal) throw new Error("terminal missing");
    const rect = terminal.getBoundingClientRect();
    terminal.dispatchEvent(
      new MouseEvent("contextmenu", {
        bubbles: true,
        cancelable: true,
        clientX: rect.left + rect.width / 2,
        clientY: rect.top + rect.height / 2,
        button: 2,
      }),
    );
  });
  const item = await b.$(`[role="menuitem"]*=${label}`);
  await item.waitForExist({ timeout: 5_000 });
  await item.click();
}

test("terminal clipboard round-trips through the native system clipboard", async () => {
  const b = getApp();
  await createProject(b, `Clipboard ${Date.now()}`);
  await openTerminal(b);

  const terminal = await b.$(".xterm");
  await terminal.waitForExist({ timeout: 20_000 });
  await terminal.click();

  const pasteMarker = `clipboard-paste-${Date.now()}`;
  await writeSystemClipboard(`printf '${pasteMarker}'`);
  await runClipboardCommand(b, "Paste");
  await b.keys(["Enter"]);
  await b.waitUntil(async () => (await terminal.getText()).includes(pasteMarker), {
    timeout: 20_000,
    timeoutMsg: "terminal did not render output from native clipboard paste",
  });

  const copyMarker = `clipboard-copy-${Date.now()}`;
  await writeSystemClipboard(`printf '${copyMarker}'`);
  await runClipboardCommand(b, "Paste");
  await b.keys(["Enter"]);
  await b.waitUntil(async () => (await terminal.getText()).includes(copyMarker), {
    timeout: 20_000,
    timeoutMsg: "terminal did not render copy-test output",
  });

  await runClipboardCommand(b, "Select all");
  await runClipboardCommand(b, "Copy");

  await b.waitUntil(async () => (await readSystemClipboard()).includes(copyMarker), {
    timeout: 10_000,
    timeoutMsg: "native clipboard did not contain copied terminal output",
  });
  expect(await readSystemClipboard()).toContain(copyMarker);
}, 120_000);
