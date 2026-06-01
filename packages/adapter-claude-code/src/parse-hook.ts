import type { HookEvent, HookEventType } from "@athing/sdk";

interface RawClaudePayload {
  session_id?: string;
  hook_event_name?: string;
  tool_name?: string;
  permission_request?: unknown;
  [key: string]: unknown;
}

const HOOK_NAME_MAP: Record<string, HookEventType> = {
  SessionStart: "SessionStart",
  UserPromptSubmit: "UserPromptSubmit",
  PostToolUse: "PostToolUse",
  PermissionRequest: "PermissionRequest",
  Stop: "Stop",
  SessionEnd: "SessionEnd",
};

function inferType(raw: RawClaudePayload): HookEventType {
  if (raw.hook_event_name && HOOK_NAME_MAP[raw.hook_event_name]) {
    return HOOK_NAME_MAP[raw.hook_event_name]!;
  }
  if ("tool_name" in raw && raw.tool_name) return "PostToolUse";
  if ("permission_request" in raw) return "PermissionRequest";
  return "SessionStart";
}

export function parseHook(raw: unknown): HookEvent {
  const payload = (typeof raw === "object" && raw !== null ? raw : {}) as RawClaudePayload;
  const sessionId = String(payload.session_id ?? "");
  const type = inferType(payload);
  return { sessionId, type, payload: raw };
}
