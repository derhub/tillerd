import { Input } from "~/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/components/ui/select";
import { Switch } from "~/components/ui/switch";
import {
  useBoolGlobalSetting,
  useGlobalSetting,
  useNumberGlobalSetting,
} from "~/lib/settings/context";
import {
  TERMINAL_CURSOR_STYLES,
  DEFAULT_TERMINAL_CURSOR_STYLE,
  DEFAULT_TERMINAL_FONT_FAMILY,
  DEFAULT_TERMINAL_FONT_SIZE,
  DEFAULT_TERMINAL_LINE_HEIGHT,
  DEFAULT_TERMINAL_SCROLLBACK,
  isTerminalCursorStyle,
  TERMINAL_CONFIRM_PASTE_KEY,
  TERMINAL_COPY_ON_SELECT_KEY,
  TERMINAL_CURSOR_BLINK_KEY,
  TERMINAL_CURSOR_STYLE_KEY,
  TERMINAL_FONT_FAMILY_KEY,
  TERMINAL_FONT_SIZE_KEY,
  TERMINAL_LINE_HEIGHT_KEY,
  TERMINAL_SCHEME_KEY,
  TERMINAL_SCROLLBACK_KEY,
} from "~/lib/settings/keys";
import { DEFAULT_TERMINAL_SCHEME, TERMINAL_SCHEME_NAMES } from "~/lib/settings/terminal-schemes";

const ROW_CLASS = "flex items-center justify-between gap-3";

// Parses a number input's raw text, ignoring an in-progress edit (empty string, a bare "-",
// a trailing ".") rather than committing 0 or NaN mid-keystroke.
function parseNumberInput(raw: string): number | null {
  if (raw === "") return null;
  const next = Number(raw);
  return Number.isNaN(next) ? null : next;
}

// All fields apply to mounted and new terminals live via the shared settings store (reactive
// path; see TerminalPane/DesktopTerminalPane) and persist across relaunch -- the scheme
// selector is unchanged from the retired popover.
export function TerminalSection() {
  const { value: scheme, setValue: setScheme } = useGlobalSetting(
    TERMINAL_SCHEME_KEY,
    DEFAULT_TERMINAL_SCHEME,
  );
  const { value: fontFamily, setValue: setFontFamily } = useGlobalSetting(
    TERMINAL_FONT_FAMILY_KEY,
    DEFAULT_TERMINAL_FONT_FAMILY,
  );
  const { value: fontSize, setValue: setFontSize } = useNumberGlobalSetting(
    TERMINAL_FONT_SIZE_KEY,
    DEFAULT_TERMINAL_FONT_SIZE,
  );
  const { value: lineHeight, setValue: setLineHeight } = useNumberGlobalSetting(
    TERMINAL_LINE_HEIGHT_KEY,
    DEFAULT_TERMINAL_LINE_HEIGHT,
  );
  const { value: scrollback, setValue: setScrollback } = useNumberGlobalSetting(
    TERMINAL_SCROLLBACK_KEY,
    DEFAULT_TERMINAL_SCROLLBACK,
  );
  const { value: rawCursorStyle, setValue: setRawCursorStyle } = useGlobalSetting(
    TERMINAL_CURSOR_STYLE_KEY,
    DEFAULT_TERMINAL_CURSOR_STYLE,
  );
  const cursorStyle = isTerminalCursorStyle(rawCursorStyle)
    ? rawCursorStyle
    : DEFAULT_TERMINAL_CURSOR_STYLE;
  const { value: cursorBlink, setValue: setCursorBlink } = useBoolGlobalSetting(
    TERMINAL_CURSOR_BLINK_KEY,
    true,
  );
  const { value: copyOnSelect, setValue: setCopyOnSelect } = useBoolGlobalSetting(
    TERMINAL_COPY_ON_SELECT_KEY,
    false,
  );
  const { value: confirmPaste, setValue: setConfirmPaste } = useBoolGlobalSetting(
    TERMINAL_CONFIRM_PASTE_KEY,
    false,
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
      <div className={ROW_CLASS}>
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

      <div className={ROW_CLASS}>
        <span className="text-foreground">Font family</span>
        <Input
          aria-label="Terminal font family"
          className="w-48"
          value={fontFamily}
          onChange={(e) => setFontFamily(e.target.value)}
        />
      </div>

      <div className={ROW_CLASS}>
        <span className="text-foreground">Font size</span>
        <Input
          type="number"
          min={1}
          aria-label="Terminal font size"
          className="w-20"
          value={fontSize}
          onChange={(e) => {
            const next = parseNumberInput(e.target.value);
            if (next !== null) setFontSize(next);
          }}
        />
      </div>

      <div className={ROW_CLASS}>
        <span className="text-foreground">Line height</span>
        <Input
          type="number"
          min={0.5}
          step={0.1}
          aria-label="Terminal line height"
          className="w-20"
          value={lineHeight}
          onChange={(e) => {
            const next = parseNumberInput(e.target.value);
            if (next !== null) setLineHeight(next);
          }}
        />
      </div>

      <div className={ROW_CLASS}>
        <span className="text-foreground">Scrollback</span>
        <Input
          type="number"
          min={0}
          aria-label="Terminal scrollback"
          className="w-24"
          value={scrollback}
          onChange={(e) => {
            const next = parseNumberInput(e.target.value);
            if (next !== null) setScrollback(next);
          }}
        />
      </div>

      <div className={ROW_CLASS}>
        <span className="text-foreground">Cursor style</span>
        <Select value={cursorStyle} onValueChange={(value) => value && setRawCursorStyle(value)}>
          <SelectTrigger aria-label="Terminal cursor style" className="w-32">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {TERMINAL_CURSOR_STYLES.map((style) => (
              <SelectItem key={style} value={style}>
                {style}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <label className={`${ROW_CLASS} text-foreground select-none`}>
        Cursor blink
        <Switch checked={cursorBlink} onCheckedChange={setCursorBlink} />
      </label>

      <label className={`${ROW_CLASS} text-foreground select-none`}>
        Copy on select
        <Switch checked={copyOnSelect} onCheckedChange={setCopyOnSelect} />
      </label>

      <label className={`${ROW_CLASS} text-foreground select-none`}>
        Confirm before pasting multi-line clipboard
        <Switch checked={confirmPaste} onCheckedChange={setConfirmPaste} />
      </label>
    </section>
  );
}
