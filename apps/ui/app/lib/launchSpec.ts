// Typed mirror of the orchestrator's versioned launch spec (crates/orchestrator
// entities/launch_spec.rs). The visual editor works over this model and serializes
// back to the exact wire shape so `launch_template_apply_spec` / `template import`
// round-trip byte-for-byte (modulo key order). There is no per-item cwd/env slot in
// v1 -- environment is a property of the library Command, not the launch item.

export const CURRENT_SPEC_VERSION = 1;

// Untagged in Rust: a library reference or an inline executable. The editor's
// command picker only produces `LibraryRef`; inline refs from an existing spec are
// preserved verbatim so an edit never silently drops them.
export type CommandRef = { library_ref: string } | { executable: string; args: string[] };

export interface LaunchItem {
  target: string;
  // Optional in the wire spec; the orchestrator mints one when absent. Omitted from
  // output when undefined rather than emitted as null, matching a hand-written spec.
  placement?: string;
  command: CommandRef;
}

export interface LaunchSpec {
  version: number;
  items: LaunchItem[];
}

export function isLibraryRef(ref: CommandRef): ref is { library_ref: string } {
  return "library_ref" in ref;
}

export function emptySpec(): LaunchSpec {
  return { version: CURRENT_SPEC_VERSION, items: [] };
}

export function newLibraryItem(commandId = ""): LaunchItem {
  return { target: "terminal", command: { library_ref: commandId } };
}

// Tolerant parse into the editor model. Throws on structurally invalid JSON or a
// missing/zero version -- mirrors the orchestrator's parse_spec guard so the editor
// rejects the same blobs the backend would.
export function parseLaunchSpec(json: string): LaunchSpec {
  const raw: unknown = JSON.parse(json);
  if (typeof raw !== "object" || raw === null) {
    throw new Error("launch spec must be an object");
  }
  const obj = raw as Record<string, unknown>;
  if (typeof obj.version !== "number" || obj.version < 1) {
    throw new Error("launch spec missing a valid version");
  }
  const itemsRaw = Array.isArray(obj.items) ? obj.items : [];
  const items = itemsRaw.map(parseItem);
  return { version: obj.version, items };
}

function parseItem(raw: unknown): LaunchItem {
  const obj = (typeof raw === "object" && raw !== null ? raw : {}) as Record<string, unknown>;
  const target = typeof obj.target === "string" ? obj.target : "terminal";
  const placement = typeof obj.placement === "string" ? obj.placement : undefined;
  const command = parseCommand(obj.command);
  return placement !== undefined ? { target, placement, command } : { target, command };
}

function parseCommand(raw: unknown): CommandRef {
  const obj = (typeof raw === "object" && raw !== null ? raw : {}) as Record<string, unknown>;
  if (typeof obj.executable === "string") {
    const args = Array.isArray(obj.args)
      ? obj.args.filter((a): a is string => typeof a === "string")
      : [];
    return { executable: obj.executable, args };
  }
  return { library_ref: typeof obj.library_ref === "string" ? obj.library_ref : "" };
}

// Serialize back to the exact wire object. Key order and omitted-when-undefined
// placement match a hand-written spec so a parse -> serialize round-trip is lossless.
export function serializeLaunchSpec(spec: LaunchSpec): string {
  return JSON.stringify(toWire(spec));
}

export function toWire(spec: LaunchSpec): Record<string, unknown> {
  return {
    version: spec.version,
    items: spec.items.map((item) => {
      const wire: Record<string, unknown> = { target: item.target };
      if (item.placement !== undefined) wire.placement = item.placement;
      wire.command = item.command;
      return wire;
    }),
  };
}

// One error string per invalid item (empty = valid). An item is invalid when its
// library reference is empty (no command picked) -- the editor blocks apply on any.
export function validateSpec(spec: LaunchSpec): string[] {
  const errors: string[] = [];
  spec.items.forEach((item, i) => {
    if (isLibraryRef(item.command) && item.command.library_ref.trim() === "") {
      errors.push(`Item ${i + 1}: no command selected`);
    }
  });
  return errors;
}
