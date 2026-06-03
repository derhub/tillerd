import type { ExitQualifier, SessionStatus } from "./types/events";

const CRASHED_QUALIFIERS = new Set<ExitQualifier>([
  "error",
  "killed",
  "faulted",
  "hangup",
  "interrupted",
  "resource-exceeded",
  "unknown",
]);

export function exitToStatus(qualifier: ExitQualifier): SessionStatus {
  return CRASHED_QUALIFIERS.has(qualifier) ? "crashed" : "DONE";
}

export function isRecoverable(qualifier: ExitQualifier): boolean {
  return CRASHED_QUALIFIERS.has(qualifier);
}

export function qualifierToCoarse(qualifier: ExitQualifier): "user" | "clean" | "unexpected" {
  if (qualifier === "stopped-by-request") return "user";
  if (qualifier === "ok") return "clean";
  return "unexpected";
}
