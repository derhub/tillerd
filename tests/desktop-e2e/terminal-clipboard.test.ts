import { expect, test } from "bun:test";

import { createProject, openTerminal } from "./helpers";
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

test("terminal copy writes the native system clipboard", async () => {
  const b = getApp();
  await createProject(b, `Clipboard copy ${Date.now()}`);
  await openTerminal(b);

  const marker = `clipboard-copy-${Date.now()}`;
  const terminal = await b.$(".xterm");
  await terminal.waitForExist({ timeout: 20_000 });
  await terminal.click();
  await b.keys(["Enter"]);
  await b.keys(["p", "r", "i", "n", "t", "f", " ", "'", marker, "'"]);
  await b.keys(["Enter"]);
  await b.waitUntil(async () => (await terminal.getText()).includes(marker), {
    timeout: 20_000,
    timeoutMsg: "terminal did not render copy-test output",
  });

  const size = await terminal.getSize();
  const location = await terminal.getLocation();
  await b.performActions([
    {
      type: "pointer",
      id: "mouse",
      parameters: { pointerType: "mouse" },
      actions: [
        { type: "pointerMove", x: location.x + 8, y: location.y + 8 },
        { type: "pointerDown", button: 0 },
        {
          type: "pointerMove",
          x: location.x + size.width - 8,
          y: location.y + size.height - 8,
          duration: 300,
        },
        { type: "pointerUp", button: 0 },
      ],
    },
  ]);
  await b.keys(process.platform === "darwin" ? ["Meta", "c"] : ["Control", "c"]);

  expect(await readSystemClipboard()).toContain(marker);
}, 120_000);
test("terminal paste reads the native system clipboard", async () => {
  const b = getApp();
  await createProject(b, `Clipboard ${Date.now()}`);
  await openTerminal(b);

  const marker = `clipboard-output-${Date.now()}`;
  await writeSystemClipboard(`printf '${marker}'`);

  const terminal = await b.$(".xterm");
  await terminal.waitForExist({ timeout: 20_000 });
  await terminal.click();
  await b.keys(process.platform === "darwin" ? ["Meta", "v"] : ["Control", "v"]);
  await b.keys(["Enter"]);

  await b.waitUntil(async () => (await terminal.getText()).includes(marker), {
    timeout: 20_000,
    timeoutMsg: "terminal did not render output from native clipboard paste",
  });
  expect((await terminal.getText()).includes(marker)).toBe(true);
}, 120_000);
