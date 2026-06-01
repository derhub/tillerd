import * as v from "valibot";

export const HookEventTypeSchema = v.union([
  v.literal("SessionStart"),
  v.literal("UserPromptSubmit"),
  v.literal("PostToolUse"),
  v.literal("PermissionRequest"),
  v.literal("Stop"),
  v.literal("SessionEnd"),
]);

export const HookEventSchema = v.object({
  sessionId: v.string(),
  type: HookEventTypeSchema,
  payload: v.optional(v.unknown()),
});

export const ToolUseContentSchema = v.object({
  kind: v.literal("tool_use"),
  sessionId: v.string(),
  toolName: v.string(),
  toolInput: v.unknown(),
});

export const EditContentSchema = v.object({
  kind: v.literal("edit"),
  sessionId: v.string(),
  filePath: v.string(),
  oldContent: v.optional(v.string()),
  newContent: v.optional(v.string()),
});

export const UsageContentSchema = v.object({
  kind: v.literal("usage"),
  sessionId: v.string(),
  inputTokens: v.number(),
  outputTokens: v.number(),
  costUsd: v.optional(v.number()),
});

export const ContentEventSchema = v.union([
  ToolUseContentSchema,
  EditContentSchema,
  UsageContentSchema,
]);

export const LaunchConfigSchema = v.object({
  command: v.string(),
  args: v.array(v.string()),
  flags: v.array(v.string()),
});

export const HookInstallSpecSchema = v.object({
  settingsPath: v.string(),
  notifyScriptPath: v.string(),
  events: v.array(HookEventTypeSchema),
});

export const AdapterConfigSchema = v.object({
  name: v.string(),
  launch: LaunchConfigSchema,
  hookInstall: HookInstallSpecSchema,
  cliVersionRange: v.string(),
});
