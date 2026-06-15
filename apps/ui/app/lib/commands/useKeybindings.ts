import { useEffect, useMemo } from "react";

import { useGlobalSetting } from "~/lib/settings/context";
import {
  DEFAULT_LEADER,
  KEYBINDINGS_LEADER_KEY,
  KEYBINDINGS_OVERRIDES_KEY,
  KEYBINDINGS_PRESET_KEY,
} from "~/lib/settings/keys";
import {
  DEFAULT_PRESET,
  eventToAccelerator,
  isPresetName,
  parseOverrides,
  resolveBindings,
  type Accelerator,
} from "./keybindings";
import { useCommands } from "./registry";

/** The resolved action -> accelerator map from the active preset and per-action overrides. */
export function useResolvedBindings(): Map<string, Accelerator> {
  const { value: presetRaw } = useGlobalSetting(KEYBINDINGS_PRESET_KEY, DEFAULT_PRESET);
  const { value: overridesRaw } = useGlobalSetting(KEYBINDINGS_OVERRIDES_KEY, "{}");
  return useMemo(() => {
    const preset = isPresetName(presetRaw) ? presetRaw : DEFAULT_PRESET;
    return resolveBindings(preset, parseOverrides(overridesRaw));
  }, [presetRaw, overridesRaw]);
}

/** The configured leader-key chord. */
export function useLeaderBinding(): string {
  return useGlobalSetting(KEYBINDINGS_LEADER_KEY, DEFAULT_LEADER).value;
}

function isCaptureTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable) return true;
  // A terminal surface owns the keystroke while focused — leave it alone.
  return target.closest(".xterm") != null || target.closest("[role=textbox]") != null;
}

/**
 * Fire an action from its configured accelerator while no editable surface (input, textarea, or a
 * focused terminal) holds focus. The leader key is excluded — it has its own native path.
 */
export function useGlobalShortcuts(bindings: Map<string, Accelerator>): void {
  const commands = useCommands();
  useEffect(() => {
    const byId = new Map(commands.map((c) => [c.id, c]));
    const onKey = (e: KeyboardEvent) => {
      if (isCaptureTarget(e.target)) return;
      const accel = eventToAccelerator(e);
      if (!accel) return;
      for (const [id, bound] of bindings) {
        if (bound !== accel) continue;
        const command = byId.get(id);
        if (command) {
          e.preventDefault();
          command.run();
        }
        return;
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [bindings, commands]);
}
