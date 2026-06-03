import type { HookEvent, ContentEvent } from "./events";
import type { Logger } from "./logger";

export interface LaunchConfig {
  command: string;
  args: string[];
  flags: string[];
}

export interface AgentDefinition {
  readonly name: string;
  readonly launch: LaunchConfig;
  readonly cliVersionRange: string;
  /** Raw bytes (as a string) the engine writes to cancel an in-progress turn. */
  readonly interruptSequence: string;
  /** Resolve the launchable command for this agent (absolute path or name). */
  resolveCommand(): string;
  installHooks(notifyCommand: string, logger: Logger): void;
  uninstallHooks(logger: Logger): void;
  parseHook(raw: unknown): HookEvent;
  transcriptPath(sessionId: string, cwd: string): string;
  parseTranscriptEntry(line: string): ContentEvent | null;
}
