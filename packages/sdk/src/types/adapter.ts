import type { HookEvent, ContentEvent } from "./events";

export interface LaunchConfig {
  command: string;
  args: string[];
  flags: string[];
}

/**
 * Declarative policy for resolving the agent binary. The adapter owns the policy
 * data; the host performs the lookup I/O and supplies the resolved command to the
 * engine as a startup value.
 */
export interface BinaryResolutionSpec {
  /** Environment variable that, when set, overrides resolution with an explicit path. */
  readonly overrideEnvVar: string;
  /** Bare binary name to resolve via the host PATH. */
  readonly binaryName: string;
  /** Fallback install locations, in order; a leading `~/` denotes the user home. */
  readonly commonLocations: readonly string[];
}

export interface AgentDefinition {
  readonly name: string;
  readonly launch: LaunchConfig;
  readonly cliVersionRange: string;
  /** Raw bytes (as a string) the engine writes to cancel an in-progress turn. */
  readonly interruptSequence: string;
  /** Declarative binary-resolution policy; the host performs the lookup I/O. */
  readonly binaryResolution: BinaryResolutionSpec;
  parseHook(raw: unknown): HookEvent;
  transcriptPath(sessionId: string, cwd: string, agentHome: string): string;
  parseTranscriptEntry(line: string): ContentEvent | null;
}
