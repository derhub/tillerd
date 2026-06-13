/** One supervised service's state, mirroring the host's read-only health view. */
export type ServiceState = "starting" | "ready" | "draining" | "versionMismatch" | "unavailable";

/** One service's observed health, read from its manifest by the host. */
export interface ServiceHealth {
  name: string;
  /** Running version, or `null` when the service is unavailable. */
  version: string | null;
  state: ServiceState;
}

export const SERVICE_HEALTH_METHOD = "service_health";

/** A service is healthy only when it is live at the expected version. */
export function isServiceHealthy(health: ServiceHealth): boolean {
  return health.state === "ready";
}
