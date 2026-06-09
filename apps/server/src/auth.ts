//! WebSocket upgrade authorization.

/**
 * Cross-site WebSocket hijacking guard. Browsers attach an `Origin` header to every
 * WebSocket handshake that page JavaScript cannot forge, so an allowlist of trusted
 * origins stops a hostile page from opening a socket to the local server and driving
 * the agent. Native clients (desktop shell, CLI) send no `Origin` and are allowed.
 */
export function isOriginAllowed(origin: string | null, allowed: ReadonlySet<string>): boolean {
  if (origin === null) return true; // non-browser client — no Origin header to check
  return allowed.has(origin);
}

/**
 * Trusted origins: always the server's own loopback origins, plus any comma-separated
 * extras from `TILLERD_ALLOWED_ORIGINS` (e.g. a dev UI on another port).
 */
export function parseAllowedOrigins(raw: string | undefined, port: number): Set<string> {
  const origins = new Set([`http://localhost:${port}`, `http://127.0.0.1:${port}`]);
  for (const entry of (raw ?? "").split(",")) {
    const trimmed = entry.trim();
    if (trimmed) origins.add(trimmed);
  }
  return origins;
}
