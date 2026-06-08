import type { HookEvent, ContentEvent } from "./types/events";

export function hookEventToContent(event: HookEvent): ContentEvent | null {
  if (event.type === "PostToolUse") {
    return {
      kind: "tool_use",
      sessionId: event.sessionId,
      toolName: event.payload.toolName,
      toolInput: event.payload.toolInput,
    };
  }
  return null;
}
