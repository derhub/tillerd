import { defineSetup } from "@athing/sdk";
import type { SetupContext } from "@athing/sdk";

const SETTINGS_FILE = "settings.json";
const HOOK_MARKER = "athing-notify";
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

function hasMarker(entries: HookEntry[]): boolean {
  return entries.some((entry) => entry.hooks.some((h) => h.command.includes(HOOK_MARKER)));
}

function hookCommand(ctx: SetupContext): string {
  if (ctx.gateUrl) {
    // Gate mode: inline curl with bearer auth; shell vars are resolved at hook runtime.
    // HOOK_MARKER (-A athing-notify) enables idempotency and uninstall detection.
    return (
      `curl -sS --max-time 5 -A ${HOOK_MARKER}` +
      ` -H "content-type: application/json"` +
      ` -H "Authorization: Bearer $ATHING_SESSION_TOKEN"` +
      ` -H "x-session-id: $ATHING_SESSION_ID"` +
      ` --data-binary @-` +
      ` "$ATHING_GATE_URL/hook/$ATHING_SESSION_ID"` +
      ` >/dev/null 2>&1 || true`
    );
  }
  return ctx.notifyCommand;
}

export const setup = defineSetup({
  async install(ctx) {
    const path = settingsPath(ctx.agentHome);
    const settings = await readSettings(ctx, path);
    const hooks = settings.hooks ?? {};

    const pending = HOOK_EVENTS.filter((event) => !hasMarker(hooks[event] ?? []));
    if (pending.length === 0) {
      ctx.logger.debug("hooks already installed", { path });
      return;
    }

    const command = hookCommand(ctx);

    for (const event of pending) {
      hooks[event] = [
        ...(hooks[event] ?? []),
        { matcher: matcherFor(event), hooks: [{ type: "command", command }] },
      ];
    }
    settings.hooks = hooks;

    await persist(ctx, path, settings);
    if (ctx.gateUrl) {
      ctx.logger.info("hooks installed", { path, events: pending, target: "gate" });
    } else {
      ctx.logger.info("hooks installed", { path, events: pending });
    }
  },

  async uninstall(ctx) {
    const path = settingsPath(ctx.agentHome);
    const settings = await readSettings(ctx, path);
    if (!settings.hooks) return;

    let changed = false;
    for (const event of HOOK_EVENTS) {
      const list = settings.hooks[event];
      if (!list) continue;
      const filtered = list.filter(
        (entry) => !entry.hooks.some((h) => h.command.includes(HOOK_MARKER)),
      );
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
