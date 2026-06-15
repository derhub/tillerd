import { useCallback, useEffect, useState } from "react";

import { useGlobalSetting } from "~/lib/settings/context";
import { KEYBINDINGS_OVERRIDES_KEY, KEYBINDINGS_PRESET_KEY } from "~/lib/settings/keys";
import { ACTION_TITLES, STATIC_ACTION_IDS, type ActionId } from "~/lib/commands/ids";
import {
  DEFAULT_PRESET,
  PRESET_NAMES,
  canonicalize,
  isPresetName,
  parseOverrides,
} from "~/lib/commands/keybindings";
import { useResolvedBindings } from "~/lib/commands/useKeybindings";

/** One action's binding: a draft input committed on blur / Enter; blank clears the override. */
function OverrideRow({
  id,
  resolved,
  onCommit,
}: {
  id: ActionId;
  resolved: string;
  onCommit: (id: ActionId, raw: string) => void;
}) {
  const [draft, setDraft] = useState(resolved);
  useEffect(() => setDraft(resolved), [resolved]);

  return (
    <label className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground truncate">{ACTION_TITLES[id]}</span>
      <input
        aria-label={`Binding for ${ACTION_TITLES[id]}`}
        data-testid={`kb-${id}`}
        className="w-32 bg-transparent border border-border/40 rounded-sm px-1 py-0.5 text-right tabular-nums"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => onCommit(id, draft)}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
    </label>
  );
}

/** Preset selector plus a per-action override editor. Persists to global settings. */
export function KeybindingSettings() {
  const { value: preset, setValue: setPreset } = useGlobalSetting(
    KEYBINDINGS_PRESET_KEY,
    DEFAULT_PRESET,
  );
  const { value: overridesRaw, setValue: setOverridesRaw } = useGlobalSetting(
    KEYBINDINGS_OVERRIDES_KEY,
    "{}",
  );
  const bindings = useResolvedBindings();

  const commit = useCallback(
    (id: ActionId, raw: string) => {
      const next = { ...parseOverrides(overridesRaw) };
      const trimmed = raw.trim();
      if (!trimmed) {
        delete next[id];
      } else {
        const canon = canonicalize(trimmed);
        if (!canon) return; // unparseable — leave the stored binding untouched
        next[id] = canon;
      }
      setOverridesRaw(JSON.stringify(next));
    },
    [overridesRaw, setOverridesRaw],
  );

  return (
    <div className="flex flex-col gap-2" data-testid="keybinding-settings">
      <div className="font-medium text-foreground">Keybindings</div>

      <label className="flex items-center justify-between gap-3">
        <span className="text-muted-foreground">Preset</span>
        <select
          aria-label="Keybinding preset"
          className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5"
          value={isPresetName(preset) ? preset : DEFAULT_PRESET}
          onChange={(e) => setPreset(e.target.value)}
        >
          {PRESET_NAMES.map((name) => (
            <option key={name} value={name}>
              {name}
            </option>
          ))}
        </select>
      </label>

      <div className="flex flex-col gap-1 max-h-40 overflow-y-auto pr-1">
        {STATIC_ACTION_IDS.map((id) => (
          <OverrideRow key={id} id={id} resolved={bindings.get(id) ?? ""} onCommit={commit} />
        ))}
      </div>
    </div>
  );
}
