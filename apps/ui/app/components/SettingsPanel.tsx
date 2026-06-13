import { Settings } from "lucide-react";

import { Popover, PopoverContent, PopoverTrigger } from "~/components/ui/popover";
import { useGlobalSetting, useTheme } from "~/lib/settings/context";
import { THEMES, TERMINAL_SCHEME_KEY, type Theme } from "~/lib/settings/keys";
import { DEFAULT_TERMINAL_SCHEME, TERMINAL_SCHEME_NAMES } from "~/lib/settings/terminal-schemes";

/**
 * Settings affordance for the shell's bottom-right cluster: a gear button opening a
 * non-modal popover with theme and terminal-scheme controls. Reads the reactive settings
 * state from context, so changes apply live across the app (incl. mounted terminals).
 */
export function SettingsPanel() {
  const { theme, setTheme } = useTheme();
  const { value: scheme, setValue: setScheme } = useGlobalSetting(
    TERMINAL_SCHEME_KEY,
    DEFAULT_TERMINAL_SCHEME,
  );

  return (
    <Popover>
      <PopoverTrigger
        aria-label="Settings"
        className="flex items-center justify-center rounded-sm bg-black/60 w-6 h-6 text-muted-foreground hover:text-foreground"
      >
        <Settings size={13} />
      </PopoverTrigger>
      <PopoverContent>
        <div className="flex flex-col gap-3 text-sm" data-testid="settings-panel">
          <div className="font-medium text-foreground">Settings</div>

          <label className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Theme</span>
            <select
              aria-label="Theme"
              className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5"
              value={theme}
              onChange={(e) => setTheme(e.target.value as Theme)}
            >
              {THEMES.map((t) => (
                <option key={t} value={t}>
                  {t}
                </option>
              ))}
            </select>
          </label>

          <label className="flex items-center justify-between gap-3">
            <span className="text-muted-foreground">Terminal scheme</span>
            <select
              aria-label="Terminal scheme"
              className="bg-transparent border border-border/40 rounded-sm px-1 py-0.5"
              value={scheme}
              onChange={(e) => setScheme(e.target.value)}
            >
              {TERMINAL_SCHEME_NAMES.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
          </label>
        </div>
      </PopoverContent>
    </Popover>
  );
}
