import { createContext, useContext } from "react";

import type { SettingsSource } from "~/lib/transport/settings-source";

/** Holds the resolved host settings source; `null` until resolved / off the desktop host. */
export const SettingsContext = createContext<SettingsSource | null>(null);

/** The host settings source, or `null` until resolved / off the desktop host. */
export function useSettingsContext(): SettingsSource | null {
  return useContext(SettingsContext);
}
