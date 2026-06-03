import type { Logger } from "./logger";

/**
 * Generic filesystem mechanics the host provides to an adapter's setup procedures.
 * Backup and atomic write are implemented once in the host and reused by every
 * adapter rather than reimplemented per adapter.
 */
export interface SetupFs {
  /** Read a file as text, or `null` when it does not exist. */
  readText(path: string): Promise<string | null>;
  /** Write text atomically (temp file + rename). */
  writeAtomic(path: string, text: string): Promise<void>;
  /** Copy the file to a timestamped backup; no-op when the file is absent. */
  backup(path: string): Promise<void>;
  /** Whether the file exists. */
  exists(path: string): Promise<boolean>;
}

/**
 * The values the host injects into an adapter's setup procedures. The adapter owns
 * the procedure and the agent-specific content; the host supplies the notify command,
 * the resolved agent-home, a logger, and the filesystem capability.
 */
export interface SetupContext {
  /** The resolved notify command the hook should invoke. */
  readonly notifyCommand: string;
  /** The resolved agent-home directory; the adapter assembles paths from it. */
  readonly agentHome: string;
  readonly logger: Logger;
  readonly fs: SetupFs;
}

/**
 * An adapter's host setup, declared as two procedures the host invokes directly.
 * The adapter owns the full flow; the host supplies the {@link SetupContext}.
 */
export interface SetupDefinition {
  install(context: SetupContext): Promise<void>;
  uninstall(context: SetupContext): Promise<void>;
}

/** Typed identity helper for declaring an adapter's setup. No host I/O of its own. */
export function defineSetup(definition: SetupDefinition): SetupDefinition {
  return definition;
}
