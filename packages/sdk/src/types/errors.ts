export type ErrorKind =
  | "BinaryNotFound"
  | "NotAuthenticated"
  | "SpawnFailed"
  | "HookInstallFailed"
  | "TranscriptUnavailable"
  | "TransportClosed"
  | "QueueFull"
  | "Timeout"
  | "VersionUnsupported";

export class AtError extends Error {
  readonly kind: ErrorKind;

  constructor(kind: ErrorKind, message?: string) {
    super(message ?? kind);
    this.name = kind;
    this.kind = kind;
  }
}
