export type SessionStatus = "IDLE" | "WORKING" | "WAITING_INPUT" | "DONE";

export type HookEventType =
  | "SessionStart"
  | "UserPromptSubmit"
  | "PostToolUse"
  | "PermissionRequest"
  | "Stop"
  | "SessionEnd";

export interface HookEvent {
  sessionId: string;
  type: HookEventType;
  payload?: unknown;
}

export interface ToolUseContent {
  kind: "tool_use";
  sessionId: string;
  toolName: string;
  toolInput: unknown;
}

export interface EditContent {
  kind: "edit";
  sessionId: string;
  filePath: string;
  oldContent?: string;
  newContent?: string;
}

export interface UsageContent {
  kind: "usage";
  sessionId: string;
  inputTokens: number;
  outputTokens: number;
  costUsd?: number;
}

export type ContentEvent = ToolUseContent | EditContent | UsageContent;

export interface ExitEvent {
  code: number | null;
  signal: string | null;
}
