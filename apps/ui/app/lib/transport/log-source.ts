import { isDesktopHost, loadTauriCore } from "./core";
import { TauriFileSource } from "./file-source";
import type { TauriCore } from "./tauri";

/** One structured log file exposed by the host, as returned by {@link LogSource.list}. */
export interface LogFileInfo {
  name: string;
  path: string;
  size: number;
}

/**
 * Host-agnostic source the log viewer reads through. The desktop adapter is
 * {@link TauriLogSource}; a server/web adapter satisfies the same contract
 * (`list` -> index endpoint, `size` -> HEAD/content-length, `read` -> Range GET)
 * without changing the viewer.
 */
export interface LogSource {
  /** Available log files with their current byte sizes. */
  list(): Promise<LogFileInfo[]>;
  /** Byte length of `path`, or `null` when the file is absent. */
  size(path: string): Promise<number | null>;
  /** Read `length` bytes from `offset`; short at end of file. */
  read(path: string, offset: number, length: number): Promise<Uint8Array>;
}

export const LIST_LOG_FILES = "list_log_files";

/** Desktop (Tauri) {@link LogSource}: `list_log_files` plus the file read/size commands. */
export class TauriLogSource implements LogSource {
  private readonly files: TauriFileSource;

  constructor(private readonly core: TauriCore) {
    this.files = new TauriFileSource(core);
  }

  list(): Promise<LogFileInfo[]> {
    return this.core.invoke<LogFileInfo[]>(LIST_LOG_FILES);
  }

  size(path: string): Promise<number | null> {
    return this.files.size(path);
  }

  read(path: string, offset: number, length: number): Promise<Uint8Array> {
    return this.files.read(path, offset, length);
  }
}

/**
 * Resolve the log source for the current host. Returns `null` off the desktop
 * host: the server/web adapter is deferred, and the viewer renders a
 * desktop-only state until it lands.
 */
export async function loadLogSource(): Promise<LogSource | null> {
  if (!isDesktopHost()) return null;
  return new TauriLogSource(await loadTauriCore());
}
