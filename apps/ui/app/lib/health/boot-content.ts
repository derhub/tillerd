import type { OrchestratorPhase } from "./aggregate";

export type BootContent = "content" | "skeleton" | "blank";

export function bootContent(orchestrator: OrchestratorPhase, graceElapsed: boolean): BootContent {
  if (orchestrator === "booting") return graceElapsed ? "skeleton" : "blank";
  return "content";
}
