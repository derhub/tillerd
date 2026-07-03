import { ensureResult } from "./readiness";
import { commands } from "./tauri_bindings.gen";

export function windowOpen(label: string, query: string): Promise<void> {
  return commands
    .windowOpen({ label, query })
    .then(ensureResult)
    .then(() => undefined);
}

export function windowFocus(label: string): Promise<void> {
  return commands
    .windowFocus({ label })
    .then(ensureResult)
    .then(() => undefined);
}

export function windowClose(label: string): Promise<void> {
  return commands
    .windowClose({ label })
    .then(ensureResult)
    .then(() => undefined);
}
