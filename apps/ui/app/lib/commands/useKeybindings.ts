import React from "react";

import { useGlobalSetting } from "~/lib/settings/context";
import {
  DEFAULT_LEADER,
  KEYBINDINGS_LEADER_KEY,
  KEYBINDINGS_OVERRIDES_KEY,
  KEYBINDINGS_PRESET_KEY,
} from "~/lib/settings/keys";

import { readContext } from "./context";
import {
  DEFAULT_PRESET,
  eventToAccelerator,
  isPresetName,
  parseOverrides,
  resolveBindings,
  type Accelerator,
} from "./keybindings";
import { useCommands } from "./registry";
import { evaluateWhen } from "./when";

export function useResolvedBindings(): Map<string, Accelerator> {
  const { value: presetRaw } = useGlobalSetting(KEYBINDINGS_PRESET_KEY, DEFAULT_PRESET);
  const { value: overridesRaw } = useGlobalSetting(KEYBINDINGS_OVERRIDES_KEY, "{}");
  return React.useMemo(() => {
    const preset = isPresetName(presetRaw) ? presetRaw : DEFAULT_PRESET;
    return resolveBindings(preset, parseOverrides(overridesRaw));
  }, [presetRaw, overridesRaw]);
}

export function useLeaderBinding(): string {
  return useGlobalSetting(KEYBINDINGS_LEADER_KEY, DEFAULT_LEADER).value;
}

function isCaptureTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable) return true;
  // Terminal surface owns keystrokes while focused.
  return target.closest(".xterm") != null || target.closest("[role=textbox]") != null;
}

export function useGlobalShortcuts(bindings: Map<string, Accelerator>): void {
  const commands = useCommands();
  React.useEffect(() => {
    const byId = new Map(commands.map((c) => [c.id, c]));
    const onKey = (e: KeyboardEvent) => {
      if (isCaptureTarget(e.target)) return;
      const accel = eventToAccelerator(e);
      if (!accel) return;
      for (const [id, bound] of bindings) {
        if (bound !== accel) continue;
        const command = byId.get(id);
        // A binding only fires when its command is available in the current context.
        if (command && evaluateWhen(command.when, readContext())) {
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
