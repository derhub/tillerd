import type { OrchestratorPhase } from "./aggregate";

/** What the shell's service-dependent content region should render during boot. */
export type BootContent = "content" | "skeleton" | "blank";

/**
 * Decide the daemon-dependent content region's render while the orchestrator
 * boots. The shell and sidebar always render; only daemon-dependent content
 * waits. A skeleton appears only once the grace delay has elapsed, so content
 * resolving quickly never flashes one. Once the orchestrator is past booting
 * (ready, error, or web) the real content renders -- a failure surfaces through
 * the health indicator, not here.
 */
export function bootContent(orchestrator: OrchestratorPhase, graceElapsed: boolean): BootContent {
  if (orchestrator === "booting") return graceElapsed ? "skeleton" : "blank";
  return "content";
}
