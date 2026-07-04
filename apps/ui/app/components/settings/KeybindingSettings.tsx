import { RotateCcw } from "lucide-react";
import React from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { COMMAND_DEFS } from "~/lib/commands/defs";
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
  title,
  resolved,
  hasOverride,
  onCommit,
  onReset,
}: {
  id: string;
  title: string;
  resolved: string;
  hasOverride: boolean;
  onCommit: (id: string, raw: string) => void;
  onReset: (id: string) => void;
}) {
  // Track only the in-progress edit; the displayed value falls back to `resolved` when not editing,
  // so an external change (preset switch / commit) flows through during render -- no syncing effect.
  const [edit, setEdit] = React.useState<string | null>(null);
  const value = edit ?? resolved;

  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground truncate">{title}</span>
      <div className="flex items-center gap-1">
        <input
          aria-label={`Binding for ${title}`}
          data-testid={`kb-${id}`}
          className="w-32 bg-transparent border border-border/40 rounded-sm px-1 py-0.5 text-right tabular-nums outline-none focus-visible:ring-1 focus-visible:ring-ring"
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
        <Tooltip>
          <TooltipTrigger
            type="button"
            aria-label={`Reset ${title} to default`}
            data-testid={`kb-${id}-reset`}
            disabled={!hasOverride}
            onClick={() => onReset(id)}
            className="flex items-center justify-center w-5 h-5 rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted disabled:opacity-30 disabled:pointer-events-none transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
          >
            <RotateCcw className="size-[var(--icon-sm)]" />
          </TooltipTrigger>
          <TooltipContent>Reset to default</TooltipContent>
        </Tooltip>
      </div>
    </div>
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

  const overrides = parseOverrides(overridesRaw);
  const hasAnyOverride = Object.keys(overrides).length > 0;

  const commit = React.useCallback(
    (id: string, raw: string) => {
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

  // Reset is just "commit an empty value" -- the same delete-if-blank path `commit`
  // already takes, so a single override mechanism (settings-stored overrides JSON)
  // backs both editing and resetting; no separate wire op needed.
  const resetOne = React.useCallback((id: string) => commit(id, ""), [commit]);
  const resetAll = React.useCallback(() => setOverridesRaw("{}"), [setOverridesRaw]);

  return (
    <div className="flex flex-col gap-2" data-testid="keybinding-settings">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground">
          Keybindings
        </h2>
        <button
          type="button"
          data-testid="kb-reset-all"
          disabled={!hasAnyOverride}
          onClick={resetAll}
          className="text-[0.833rem] text-muted-foreground hover:text-foreground disabled:opacity-30 disabled:pointer-events-none transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
        >
          Reset all
        </button>
      </div>

      <label className="flex items-center justify-between gap-3">
        <span className="text-muted-foreground">Preset</span>
        <select
          aria-label="Keybinding preset"
          className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5 outline-none focus-visible:ring-1 focus-visible:ring-ring"
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

      <div className="flex flex-col gap-1 max-h-[60vh] overflow-y-auto pr-1">
        {COMMAND_DEFS.map((def) => (
          <OverrideRow
            key={def.id}
            id={def.id}
            title={def.title}
            resolved={bindings.get(def.id) ?? ""}
            hasOverride={def.id in overrides}
            onCommit={commit}
            onReset={resetOne}
          />
        ))}
      </div>
    </div>
  );
}
