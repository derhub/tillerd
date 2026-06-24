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

// Settings cross the wire as the orchestrator's raw JSON-encoded string; the JSON value is
// serialized/parsed here. Null off the desktop host.
export function loadSettingsSource(): Promise<SettingsSource | null> {
  return withDesktopCore(() => ({
    getSetting: async ({ scope, projectId, key }) => {
      const raw = await commands
        .settingGet({ scope, projectId: projectId ?? null, key })
        .then(ensureResult);
      return raw === null ? null : JSON.parse(raw);
    },
    setSetting: ({ scope, projectId, key, value }) =>
      commands
        .settingSet({ scope, projectId: projectId ?? null, key, valueJson: JSON.stringify(value) })
        .then(ensureResult)
        .then(() => undefined),
    listSettings: ({ scope, projectId }) =>
      commands.settingList({ scope, projectId: projectId ?? null }).then(ensureResult),
  }));
}
