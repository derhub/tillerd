import type { LucideIcon } from "lucide-react";

import type { Accelerator, PresetName } from "./keybindings";
import type { ContextSnapshot, WhenExpr } from "./when";

// Where a command appears. A command is projected onto each surface it is tagged
// for; the palette is the default.
export type Surface = "palette" | "titlebar" | "contextmenu" | "activitybar" | "statusbar";

// The single, static declaration of a command: identity, presentation, where it
// appears, its default keys per preset, its availability, and (for toggles) its
// checked selector. Handlers are NOT part of the definition -- they register by
// id at runtime so they can close over live application context.
export interface CommandDef {
  id: string;
  title: string;
  category?: string;
  keywords?: string[];
  icon?: LucideIcon;
  surfaces?: Surface[];
  group?: string;
  defaultKeys?: Partial<Record<PresetName, Accelerator>>;
  when?: WhenExpr;
  // Presence marks the command as a toggle; the result is its checked state,
  // computed from context and never stored separately from the source of truth.
  toggle?: (ctx: ContextSnapshot) => boolean;
}

// Target context for a command invocation, e.g. the row a context menu was
// opened on. Open shape (beyond entityId/entityKind) so a surface can carry
// extra context without widening this type for every future field.
export interface CommandArgs {
  entityId?: string;
  entityKind?: string;
  [key: string]: unknown;
}

// The arg is optional so every existing no-arg handler keeps working
// unchanged -- only surfaces that carry target context (e.g. contextmenu)
// invoke with an argument.
export type CommandHandler = (args?: CommandArgs) => void;

// A definition composed with its resolved handler and (for toggles) checked
// state. This is what surfaces consume. `run` is a no-op when no handler is
// registered for the id.
export interface Command extends CommandDef {
  run: CommandHandler;
  checked?: boolean;
}

export function surfacesOf(def: CommandDef): readonly Surface[] {
  return def.surfaces ?? ["palette"];
}

export function isOnSurface(def: CommandDef, surface: Surface): boolean {
  return surfacesOf(def).includes(surface);
}
