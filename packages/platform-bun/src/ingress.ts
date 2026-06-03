import * as fs from "node:fs";
import * as path from "node:path";
import { AtError } from "@athing/sdk";

export function notifyScriptPath(): string {
  const envBin = process.env["ATHING_NOTIFY_BIN"];
  if (envBin) {
    const abs = path.resolve(envBin);
    if (fs.existsSync(abs)) return abs;
  }

  const localBin = path.join(process.cwd(), "bin", "athing-notify");
  if (fs.existsSync(localBin)) return localBin;

  return path.join(import.meta.dir, "../../../../bin/athing-notify");
}

export function notifyCommand(): string {
  return notifyScriptPath();
}

export function prepareNotifyScript(
  target: string = notifyScriptPath(),
): { command: string; updated: boolean } {
  if (!fs.existsSync(target)) {
    throw new AtError(
      "HookInstallFailed",
      `notify client not found at ${target} — expected the committed bin/athing-notify`,
    );
  }
  try {
    fs.accessSync(target, fs.constants.X_OK);
  } catch {
    throw new AtError("HookInstallFailed", `notify client at ${target} is not executable`);
  }
  return { command: target, updated: false };
}
