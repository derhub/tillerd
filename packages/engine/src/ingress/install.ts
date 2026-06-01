import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { AtError } from "@athing/sdk";

export function notifyScriptPath(): string {
  const athingDir = process.env["ATHING_DIR"]
    ? path.resolve(process.env["ATHING_DIR"])
    : path.join(os.homedir(), ".athing");
  return path.join(athingDir, "notify.mjs");
}

export function notifyCommand(): string {
  return `bun ${notifyScriptPath()}`;
}

export function prepareNotifyScript(): { command: string; updated: boolean } {
  const target = notifyScriptPath();
  if (!fs.existsSync(target)) {
    throw new AtError(
      "HookInstallFailed",
      `notify script not found at ${target} — run: bun run build`,
    );
  }
  return { command: `bun ${target}`, updated: false };
}
