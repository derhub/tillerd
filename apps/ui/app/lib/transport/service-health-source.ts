import { SERVICE_HEALTH_METHOD, type ServiceHealth } from "@tillerd/sdk/orchestrator";

import { isDesktopHost, loadTauriCore } from "./core";
import type { TauriCore } from "./tauri";

/**
 * Host-agnostic source the health indicator reads through. The desktop adapter is
 * {@link TauriServiceHealthSource}; a server/web adapter satisfies the same
 * contract without changing the indicator. Read-only: it exposes a snapshot and
 * nothing that changes a service's lifecycle.
 */
export interface ServiceHealthSource {
  /** A read-only snapshot of every supervised service's health. */
  snapshot(): Promise<ServiceHealth[]>;
}

/** Desktop (Tauri) {@link ServiceHealthSource}: the `service_health` command. */
export class TauriServiceHealthSource implements ServiceHealthSource {
  constructor(private readonly core: TauriCore) {}

  snapshot(): Promise<ServiceHealth[]> {
    return this.core.invoke<ServiceHealth[]>(SERVICE_HEALTH_METHOD);
  }
}

/**
 * Resolve the health source for the current host. Returns `null` off the desktop
 * host: the server/web adapter is deferred, and the indicator hides until it lands.
 */
export async function loadServiceHealthSource(): Promise<ServiceHealthSource | null> {
  if (!isDesktopHost()) return null;
  return new TauriServiceHealthSource(await loadTauriCore());
}
