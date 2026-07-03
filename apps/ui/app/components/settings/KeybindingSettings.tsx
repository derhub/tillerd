import React from "react";

import { ACTION_TITLES, STATIC_ACTION_IDS, type ActionId } from "~/lib/commands/ids";
import {
  DEFAULT_PRESET,
  PRESET_NAMES,
  canonicalize,
  isPresetName,
  parseOverrides,
} from "~/lib/commands/keybindings";
import { useResolvedBindings } from "~/lib/commands/useKeybindings";
import { useGlobalSetting } from "~/lib/settings/context";
import { KEYBINDINGS_OVERRIDES_KEY, KEYBINDINGS_PRESET_KEY } from "~/lib/settings/keys";

function OverrideRow({
  id,
  resolved,
  onCommit,
}: {
  id: ActionId;
  resolved: string;
  onCommit: (id: ActionId, raw: string) => void;
}) {
  // Track only the in-progress edit; the displayed value falls back to `resolved` when not editing,
  // so an external change (preset switch / commit) flows through during render -- no syncing effect.
  const [edit, setEdit] = React.useState<string | null>(null);
  const value = edit ?? resolved;

  return (
    <label className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground truncate">{ACTION_TITLES[id]}</span>
      <input
        aria-label={`Binding for ${ACTION_TITLES[id]}`}
        data-testid={`kb-${id}`}
        className="w-32 bg-transparent border border-border/40 rounded-sm px-1 py-0.5 text-right tabular-nums"
        value={value}
        onChange={(e) => setEdit(e.target.value)}
        onBlur={() => {
          onCommit(id, value);
          setEdit(null);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
    </label>
  );
}

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

  const commit = React.useCallback(
    (id: ActionId, raw: string) => {
      const next = { ...parseOverrides(overridesRaw) };
      const trimmed = raw.trim();
      if (!trimmed) {
        delete next[id];
      } else {
        const canon = canonicalize(trimmed);
        if (!canon) return; // unparseable -- leave the stored binding untouched
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
