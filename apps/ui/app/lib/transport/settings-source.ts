import { createSettingsClient, type SettingsClient } from "@tillerd/sdk/orchestrator";

import { isDesktopHost, loadTauriCore } from "./core";

/**
 * Host-agnostic settings access the UI reads/writes through. The desktop adapter wires
 * the Tauri core; a server/web adapter satisfies the same {@link SettingsClient} contract
 * without changing any consumer. Scoped (global / project) JSON values persisted by the
 * orchestrator `setting` table.
 */
export type SettingsSource = SettingsClient;

/**
 * Resolve the settings source for the current host. Returns `null` off the desktop host:
 * the server/web adapter is deferred, and consumers fall back to defaults until it lands.
 */
export async function loadSettingsSource(): Promise<SettingsSource | null> {
  if (!isDesktopHost()) return null;
  const core = await loadTauriCore();
  return createSettingsClient({ invoke: (cmd, args) => core.invoke(cmd, args) });
}
