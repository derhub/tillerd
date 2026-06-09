import { defineSetup } from "@tillerd/sdk";
import type { SetupContext } from "@tillerd/sdk";

const SETTINGS_FILE = "settings.json";
const HOOK_MARKER = "tillerd-notify";
const HOOK_EVENTS = [
  "SessionStart",
  "UserPromptSubmit",
  "PostToolUse",
  "PermissionRequest",
  "Stop",
  "SessionEnd",
] as const;

function matcherFor(event: string): string {
  return event === "PostToolUse" ? "*" : "";
}

interface HookEntry {
  matcher: string;
  hooks: Array<{ type: string; command: string }>;
}

interface Settings {
  hooks?: Record<string, HookEntry[]>;
  [key: string]: unknown;
}

function settingsPath(agentHome: string): string {
  return `${agentHome}/${SETTINGS_FILE}`;
}

async function readSettings(ctx: SetupContext, path: string): Promise<Settings> {
  const raw = await ctx.fs.readText(path);
  if (!raw) return {};
  try {
    return JSON.parse(raw) as Settings;
  } catch {
    return {};
  }
}

async function persist(ctx: SetupContext, path: string, settings: Settings): Promise<void> {
  await ctx.fs.backup(path);
  await ctx.fs.writeAtomic(path, JSON.stringify(settings, null, 2) + "\n");
}

function isTillerdEntry(entry: HookEntry): boolean {
  return entry.hooks.some((h) => h.command.includes(HOOK_MARKER));
}

// A pre-frame-socket hook: the old gate-mode curl. Detected so re-running setup
// migrates it to the notify binary.
function isLegacyEntry(entry: HookEntry): boolean {
  return entry.hooks.some((h) => h.command.includes("curl"));
}

export const setup = defineSetup({
  async install(ctx) {
    const path = settingsPath(ctx.agentHome);
    const settings = await readSettings(ctx, path);
    const hooks = settings.hooks ?? {};
    const command = ctx.notifyCommand;

    let changed = false;
    for (const event of HOOK_EVENTS) {
      const list = hooks[event] ?? [];
      const installed = list.filter(isTillerdEntry).some((entry) => !isLegacyEntry(entry));
      if (installed) continue;
      // Drop any legacy curl hook before adding the notify binary command.
      const cleaned = list.filter((entry) => !isTillerdEntry(entry));
      cleaned.push({ matcher: matcherFor(event), hooks: [{ type: "command", command }] });
      hooks[event] = cleaned;
      changed = true;
    }

    if (!changed) {
      ctx.logger.debug("hooks already installed", { path });
      return;
    }
    settings.hooks = hooks;
    await persist(ctx, path, settings);
    ctx.logger.info("hooks installed", { path });
  },

  async uninstall(ctx) {
    const path = settingsPath(ctx.agentHome);
    const settings = await readSettings(ctx, path);
    if (!settings.hooks) return;

    let changed = false;
    for (const event of HOOK_EVENTS) {
      const list = settings.hooks[event];
      if (!list) continue;
      const filtered = list.filter((entry) => !isTillerdEntry(entry));
      if (filtered.length !== list.length) {
        settings.hooks[event] = filtered;
        changed = true;
      }
    }

    if (!changed) return;
    await persist(ctx, path, settings);
    ctx.logger.info("hooks uninstalled", { path });
  },
});
