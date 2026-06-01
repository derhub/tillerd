import type { AgentDefinition, HookEvent } from "@athing/sdk";

export const claudeCodeAdapter: AgentDefinition = {
  name: "claude-code",
  description: "Claude Code agent adapter",
};

export function parseHook(event: HookEvent): unknown {
  return event;
}
