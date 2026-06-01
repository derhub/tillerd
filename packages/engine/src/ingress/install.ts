import * as fs from "node:fs";
import * as path from "node:path";
import type { HookInstallSpec } from "@athing/sdk";
import { AtError } from "@athing/sdk";
import type { Logger } from "../logger";

const ATHING_HOOK_MARKER = "athing-notify";

interface ClaudeSettings {
  hooks?: Record<
    string,
    Array<{ matcher: string; hooks: Array<{ type: string; command: string }> }>
  >;
  [key: string]: unknown;
}

function readSettings(settingsPath: string): ClaudeSettings {
  try {
    const raw = fs.readFileSync(settingsPath, "utf8");
    return JSON.parse(raw) as ClaudeSettings;
  } catch {
    return {};
  }
}

function writeSettings(settingsPath: string, settings: ClaudeSettings): void {
  const dir = path.dirname(settingsPath);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(settingsPath, JSON.stringify(settings, null, 2) + "\n", "utf8");
}

function buildNotifyCommand(scriptPath: string): string {
  return `bun ${scriptPath}`;
}

function ensureNotifyScript(scriptPath: string): void {
  const dir = path.dirname(scriptPath);
  fs.mkdirSync(dir, { recursive: true });
  const script = `
const chunks = [];
process.stdin.on('data', c => chunks.push(c));
process.stdin.on('end', async () => {
  const raw = Buffer.concat(chunks).toString('utf8');
  const bridgeUrl = process.env.ATHING_BRIDGE_URL;
  const token = process.env.ATHING_SESSION_TOKEN;
  const sessionId = process.env.ATHING_SESSION_ID;
  if (!bridgeUrl) return;
  try {
    await fetch(bridgeUrl, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-session-token': token ?? '',
        'x-session-id': sessionId ?? '',
      },
      body: raw,
    });
  } catch (_) {}
});
`.trimStart();
  fs.writeFileSync(scriptPath, script, "utf8");
}

export function installHooks(spec: HookInstallSpec, logger: Logger): void {
  const settingsPath = spec.settingsPath.replace("~", process.env["HOME"] ?? "~");
  const scriptPath = spec.notifyScriptPath.replace("~", process.env["HOME"] ?? "~");

  try {
    ensureNotifyScript(scriptPath);
    const command = buildNotifyCommand(scriptPath);
    const settings = readSettings(settingsPath);

    if (!settings.hooks) settings.hooks = {};

    let changed = false;
    for (const event of spec.events) {
      if (!settings.hooks[event]) settings.hooks[event] = [];
      const existing = settings.hooks[event]!;
      const alreadyInstalled = existing.some((entry) =>
        entry.hooks.some((h) => h.command.includes(ATHING_HOOK_MARKER)),
      );
      if (!alreadyInstalled) {
        existing.push({
          matcher: event === "PostToolUse" ? "*" : "",
          hooks: [{ type: "command", command }],
        });
        changed = true;
      }
    }

    if (changed) {
      writeSettings(settingsPath, settings);
      logger.info("hooks installed", { settingsPath, events: spec.events });
    } else {
      logger.debug("hooks already installed", { settingsPath });
    }
  } catch (err) {
    throw new AtError("HookInstallFailed", String(err));
  }
}

export function uninstallHooks(spec: HookInstallSpec, logger: Logger): void {
  const settingsPath = spec.settingsPath.replace("~", process.env["HOME"] ?? "~");
  try {
    const settings = readSettings(settingsPath);
    if (!settings.hooks) return;

    let changed = false;
    for (const event of spec.events) {
      if (!settings.hooks[event]) continue;
      const before = settings.hooks[event]!.length;
      settings.hooks[event] = settings.hooks[event]!.filter(
        (entry) => !entry.hooks.some((h) => h.command.includes(ATHING_HOOK_MARKER)),
      );
      if (settings.hooks[event]!.length !== before) changed = true;
    }

    if (changed) {
      writeSettings(settingsPath, settings);
      logger.info("hooks uninstalled", { settingsPath });
    }
  } catch (err) {
    logger.warn("hooks uninstall failed", { err: String(err) });
  }
}
