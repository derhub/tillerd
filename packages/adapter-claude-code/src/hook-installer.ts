import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import type { Logger } from "@athing/sdk";

export const DEFAULT_SETTINGS_PATH = path.join(os.homedir(), ".claude", "settings.json");
const HOOK_MARKER = "notify.mjs";
const HOOK_EVENTS = [
  "SessionStart",
  "UserPromptSubmit",
  "PostToolUse",
  "PermissionRequest",
  "Stop",
  "SessionEnd",
] as const;

interface ClaudeSettings {
  hooks?: Record<
    string,
    Array<{ matcher: string; hooks: Array<{ type: string; command: string }> }>
  >;
  [key: string]: unknown;
}

function readSettings(settingsPath: string): ClaudeSettings {
  try {
    return JSON.parse(fs.readFileSync(settingsPath, "utf8")) as ClaudeSettings;
  } catch {
    return {};
  }
}

function backupSettings(settingsPath: string): void {
  if (!fs.existsSync(settingsPath)) return;
  const ts = new Date()
    .toISOString()
    .replace(/:/g, "-")
    .replace(/\.\d+Z$/, "Z");
  fs.copyFileSync(settingsPath, `${settingsPath}.athing-backup-${ts}`);
}

function writeSettings(settings: ClaudeSettings, settingsPath: string): void {
  fs.mkdirSync(path.dirname(settingsPath), { recursive: true });
  backupSettings(settingsPath);
  const tmp = `${settingsPath}.athing-tmp`;
  fs.writeFileSync(tmp, JSON.stringify(settings, null, 2) + "\n", "utf8");
  fs.renameSync(tmp, settingsPath);
}

export function installHooks(
  notifyCommand: string,
  logger: Logger,
  settingsPath = DEFAULT_SETTINGS_PATH,
): void {
  const settings = readSettings(settingsPath);
  if (!settings.hooks) settings.hooks = {};

  const pending = HOOK_EVENTS.filter((event) => {
    const existing = settings.hooks![event] ?? [];
    return !existing.some((entry) => entry.hooks.some((h) => h.command.includes(HOOK_MARKER)));
  });

  if (pending.length === 0) {
    logger.debug("hooks already installed", { settingsPath });
    return;
  }

  for (const event of pending) {
    if (!settings.hooks![event]) settings.hooks![event] = [];
    settings.hooks![event]!.push({
      matcher: event === "PostToolUse" ? "*" : "",
      hooks: [{ type: "command", command: notifyCommand }],
    });
  }

  writeSettings(settings, settingsPath);
  logger.info("hooks installed", { settingsPath, events: pending });
}

export function uninstallHooks(logger: Logger, settingsPath = DEFAULT_SETTINGS_PATH): void {
  const settings = readSettings(settingsPath);
  if (!settings.hooks) return;

  let changed = false;
  for (const event of HOOK_EVENTS) {
    if (!settings.hooks[event]) continue;
    const before = settings.hooks[event]!.length;
    settings.hooks[event] = settings.hooks[event]!.filter(
      (entry) => !entry.hooks.some((h) => h.command.includes(HOOK_MARKER)),
    );
    if (settings.hooks[event]!.length !== before) changed = true;
  }

  if (changed) {
    writeSettings(settings, settingsPath);
    logger.info("hooks uninstalled", { settingsPath });
  }
}
