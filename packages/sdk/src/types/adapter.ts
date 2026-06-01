import type { HookEvent, ContentEvent, HookEventType } from "./events";

export interface LaunchConfig {
  command: string;
  args: string[];
  flags: string[];
}

export interface HookInstallSpec {
  settingsPath: string;
  notifyScriptPath: string;
  events: HookEventType[];
}

export interface AgentDefinition {
  readonly name: string;
  readonly launch: LaunchConfig;
  readonly hookInstall: HookInstallSpec;
  readonly cliVersionRange: string;
  parseHook(raw: unknown): HookEvent;
  transcriptPath(sessionId: string, cwd: string): string;
  parseTranscriptEntry(line: string): ContentEvent | null;
}
