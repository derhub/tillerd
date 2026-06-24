import type { SettingView } from "@tillerd/client-bindings";

import { commands, ensureResult } from "@tillerd/client-bindings";

import { withDesktopCore } from "./core";

export interface SettingsSource {
  getSetting(args: { scope: string; projectId?: string | null; key: string }): Promise<unknown>;
  setSetting(args: {
    scope: string;
    projectId?: string | null;
    key: string;
    value: unknown;
  }): Promise<void>;
  listSettings(args: { scope: string; projectId?: string | null }): Promise<SettingView[]>;
}

// `setting_get`/`setting_set` carry an arbitrary JSON `value`, which specta cannot type as a command
// parameter -- they stay on the raw `core.invoke` escape hatch. `setting_list` is fully typed, so it
// goes through the generated `commands`. Null off the desktop host.
export function loadSettingsSource(): Promise<SettingsSource | null> {
  return withDesktopCore((core) => ({
    getSetting: (args) => core.invoke("setting_get", args as Record<string, unknown>),
    setSetting: (args) =>
      core.invoke<void>("setting_set", args as unknown as Record<string, unknown>),
    listSettings: (args) =>
      commands
        .settingList({ scope: args.scope, projectId: args.projectId ?? null })
        .then(ensureResult),
  }));
}
