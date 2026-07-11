// Exit qualifiers the runtime treats as a clean stop, mirroring surface_channel.rs's exit_status().
// A clean exit shows no failure overlay and renders in the success color; anything else is a failure.
const CLEAN_EXIT_QUALIFIERS: ReadonlySet<string> = new Set(["ok", "stopped-by-request"]);

export function isCleanExit(qualifier: string): boolean {
  return CLEAN_EXIT_QUALIFIERS.has(qualifier);
}
