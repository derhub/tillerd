import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import { useGlobalSetting } from "~/lib/settings/context";
import { TERMINAL_SCHEME_KEY } from "~/lib/settings/keys";
import { DEFAULT_TERMINAL_SCHEME, TERMINAL_SCHEME_NAMES } from "~/lib/settings/terminal-schemes";

// Scheme applies to mounted terminals live (no respawn) via the shared settings store --
// unchanged from the retired popover.
export function TerminalSection() {
  const { value: scheme, setValue: setScheme } = useGlobalSetting(
    TERMINAL_SCHEME_KEY,
    DEFAULT_TERMINAL_SCHEME,
  );

  return (
    <section aria-labelledby="settings-terminal-heading" className="flex flex-col gap-3 max-w-sm">
      <h2
        id="settings-terminal-heading"
        className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground"
      >
        Terminal
      </h2>

      {/* A wrapping <label> would give the Select's hidden form-submission <input> the same
          accessible name as the trigger's own aria-label -- see AppearanceSection for the same
          note; a plain row keeps "Terminal scheme" naming exactly one element. */}
      <div className="flex items-center justify-between gap-3">
        <span className="text-foreground">Terminal scheme</span>
        <Select value={scheme} onValueChange={(value) => value && setScheme(value)}>
          <SelectTrigger aria-label="Terminal scheme" className="w-40">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TERMINAL_SCHEME_NAMES.map((name) => (
              <SelectItem key={name} value={name}>
                {name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </section>
  );
}
