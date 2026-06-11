export const SURFACE_CREATE = "surface_create";
export const SURFACE_INPUT = "surface_input";
export const SURFACE_RESIZE = "surface_resize";
export const SURFACE_DETACH = "surface_detach";
export const SURFACE_STATUS_EVENT = "surface://status";
export const SURFACE_EXIT_EVENT = "surface://exit";

export interface SurfaceStatusEvent {
  surfaceId: string;
  status: string;
}

export interface SurfaceExitEvent {
  surfaceId: string;
  qualifier: string;
}

export interface TerminalSurfaceTransport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: (payload: T) => void): Promise<() => void>;
  createByteChannel(onBytes: (bytes: Uint8Array) => void): unknown;
}

export interface CreateTerminalOptions {
  /** The session this surface belongs to. */
  sessionId: string;
  cols: number;
  rows: number;
  cwd?: string;
}

export interface TerminalSurfaceClient {
  create(opts: CreateTerminalOptions, onBytes: (bytes: Uint8Array) => void): Promise<string>;
  input(surfaceId: string, bytes: Uint8Array): Promise<void>;
  resize(surfaceId: string, cols: number, rows: number): Promise<void>;
  detach(surfaceId: string): Promise<void>;
  onStatus(handler: (e: SurfaceStatusEvent) => void): Promise<() => void>;
  onExit(handler: (e: SurfaceExitEvent) => void): Promise<() => void>;
}

export function createTerminalSurfaceClient(
  transport: TerminalSurfaceTransport,
): TerminalSurfaceClient {
  return {
    create: async (opts, onBytes) => {
      const channel = transport.createByteChannel(onBytes);
      const id = await transport.invoke<string>(SURFACE_CREATE, {
        channel,
        sessionId: opts.sessionId,
        cols: opts.cols,
        rows: opts.rows,
        cwd: opts.cwd,
      });
      return id;
    },
    input: (surfaceId, bytes) =>
      transport.invoke(SURFACE_INPUT, { surfaceId, bytes: Array.from(bytes) }),
    resize: (surfaceId, cols, rows) => transport.invoke(SURFACE_RESIZE, { surfaceId, cols, rows }),
    detach: (surfaceId) => transport.invoke(SURFACE_DETACH, { surfaceId }),
    onStatus: (handler) => transport.listen<SurfaceStatusEvent>(SURFACE_STATUS_EVENT, handler),
    onExit: (handler) => transport.listen<SurfaceExitEvent>(SURFACE_EXIT_EVENT, handler),
  };
}
