import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import { useTheme } from "~/lib/settings/context";
import { THEMES, type Theme } from "~/lib/settings/keys";

// Theme applies immediately via useTheme (document class + localStorage cache), unchanged
// from the retired popover -- first-paint restore lives in the boot script, not here.
export function AppearanceSection() {
  const { theme, setTheme } = useTheme();

  return (
    <section
      aria-labelledby="settings-appearance-heading"
      className="flex flex-col gap-3 max-w-sm"
    >
      <h2
        id="settings-appearance-heading"
        className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground"
      >
        Appearance
      </h2>

      {/* A wrapping <label> would give the Select's hidden form-submission <input> the same
          accessible name as the trigger's own aria-label (two elements answering to "Theme") --
          a plain row with the trigger's aria-label as the sole accessible name avoids that. */}
      <div className="flex items-center justify-between gap-3">
        <span className="text-foreground">Theme</span>
        <Select value={theme} onValueChange={(value) => value && setTheme(value as Theme)}>
          <SelectTrigger aria-label="Theme" className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {THEMES.map((t) => (
              <SelectItem key={t} value={t}>
                {t}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </section>
  );
}
