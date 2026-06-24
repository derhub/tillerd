import type { ServiceHealthWire } from "@tillerd/client-bindings";

import { commands } from "@tillerd/client-bindings";

import { isDesktopHost } from "./core";

export interface ServiceHealthSource {
  snapshot(): Promise<ServiceHealthWire[]>;
}

export function loadServiceHealthSource(): Promise<ServiceHealthSource | null> {
  if (!isDesktopHost()) return Promise.resolve(null);
  return Promise.resolve({ snapshot: () => commands.serviceHealth() });
}
