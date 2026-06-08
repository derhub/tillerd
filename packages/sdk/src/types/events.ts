export type SessionStatus = "IDLE" | "WORKING" | "WAITING_INPUT" | "DONE" | "crashed";

export type ExitQualifier =
  | "ok"
  | "error"
  | "stopped-by-request"
  | "killed"
  | "faulted"
  | "hangup"
  | "interrupted"
  | "resource-exceeded"
  | "unknown";

export type SignalCategory =
  | "graceful-termination"
  | "forced-termination"
  | "fault"
  | "job-control"
  | "resource"
  | "timer"
  | "user-defined"
  | "child"
  | "window"
  | "info";

export type HookKind =
  | { type: "SessionStart"; payload: { cwd?: string; client?: string; cliVersion?: string } }
  | { type: "UserPromptSubmit"; payload: { content: string; turnIndex?: number } }
  | {
      type: "PostToolUse";
      payload: { toolName: string; toolInput: unknown; toolResponse: string; turnIndex: number };
    }
  | { type: "PermissionRequest"; payload: { toolName?: string; request: unknown } }
  | { type: "Stop"; payload: { turnIndex?: number } }
  | { type: "SessionEnd"; payload: { reason?: string } };

export type HookEventType = HookKind["type"];

export type HookEvent = { sessionId: string; correlationId: string; ts: number } & HookKind;

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

export interface ExitEventRaw {
  code?: number | null;
  signal?: string | null;
  signalName?: string;
  signalMeaning?: string;
  signalCategory?: SignalCategory;
}

export interface ExitEvent {
  qualifier: ExitQualifier;
  raw?: ExitEventRaw;
}
