import React from "react";

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
  clampTerminalFontSize,
  clampTerminalLineHeight,
  clampTerminalScrollback,
  isTerminalCursorStyle,
  TERMINAL_CONFIRM_PASTE_KEY,
  TERMINAL_COPY_ON_SELECT_KEY,
  TERMINAL_CURSOR_BLINK_KEY,
  TERMINAL_CURSOR_STYLE_KEY,
  TERMINAL_FONT_FAMILY_KEY,
  TERMINAL_FONT_SIZE_KEY,
  TERMINAL_FONT_SIZE_MAX,
  TERMINAL_FONT_SIZE_MIN,
  TERMINAL_LINE_HEIGHT_KEY,
  TERMINAL_LINE_HEIGHT_MAX,
  TERMINAL_LINE_HEIGHT_MIN,
  TERMINAL_SCHEME_KEY,
  TERMINAL_SCROLLBACK_KEY,
  TERMINAL_SCROLLBACK_MAX,
  TERMINAL_SCROLLBACK_MIN,
} from "~/lib/settings/keys";
import { DEFAULT_TERMINAL_SCHEME, TERMINAL_SCHEME_NAMES } from "~/lib/settings/terminal-schemes";

const ROW_CLASS = "flex items-center justify-between gap-3";

// A numeric setting field that commits only on blur or Enter -- not per keystroke. Committing
// each keystroke pushed intermediate digits straight into every mounted terminal (typing "5000"
// into Scrollback transiently applied 5, irreversibly trimming live buffers; font size flashed
// mid-typing), and typed values bypassed the input's `min`/`max`. The draft is local while the
// user edits; on commit it is parsed and clamped to the setting's hard bounds before it reaches
// the store, so an out-of-range value (e.g. lineHeight < 1) can never be persisted or applied.
function NumberSettingField({
  label,
  ariaLabel,
  value,
  onCommit,
  clamp,
  min,
  max,
  step,
  width,
}: {
  label: string;
  ariaLabel: string;
  value: number;
  onCommit: (value: number) => void;
  clamp: (value: number) => number;
  min: number;
  max: number;
  step?: number;
  width: string;
}) {
  const [draft, setDraft] = React.useState(() => String(value));
  const editingRef = React.useRef(false);

  // Resync the draft when the committed value changes from outside this field (a sibling
  // window's write, a profile activation) while the user is not mid-edit.
  React.useEffect(() => {
    if (!editingRef.current) setDraft(String(value));
  }, [value]);

  const commit = React.useCallback(() => {
    editingRef.current = false;
    const parsed = Number(draft);
    if (draft.trim() === "" || Number.isNaN(parsed)) {
      setDraft(String(value)); // discard an unparseable in-progress edit
      return;
    }
    const clamped = clamp(parsed);
    onCommit(clamped);
    setDraft(String(clamped));
  }, [draft, value, clamp, onCommit]);

  return (
    <div className={ROW_CLASS}>
      <span className="text-foreground">{label}</span>
      <Input
        type="number"
        min={min}
        max={max}
        step={step}
        aria-label={ariaLabel}
        className={width}
        value={draft}
        onFocus={() => {
          editingRef.current = true;
        }}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            e.currentTarget.blur();
          }
        }}
      />
    </div>
  );
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

      <NumberSettingField
        label="Font size"
        ariaLabel="Terminal font size"
        value={fontSize}
        onCommit={setFontSize}
        clamp={clampTerminalFontSize}
        min={TERMINAL_FONT_SIZE_MIN}
        max={TERMINAL_FONT_SIZE_MAX}
        step={1}
        width="w-20"
      />

      <NumberSettingField
        label="Line height"
        ariaLabel="Terminal line height"
        value={lineHeight}
        onCommit={setLineHeight}
        clamp={clampTerminalLineHeight}
        min={TERMINAL_LINE_HEIGHT_MIN}
        max={TERMINAL_LINE_HEIGHT_MAX}
        step={0.1}
        width="w-20"
      />

      <NumberSettingField
        label="Scrollback"
        ariaLabel="Terminal scrollback"
        value={scrollback}
        onCommit={setScrollback}
        clamp={clampTerminalScrollback}
        min={TERMINAL_SCROLLBACK_MIN}
        max={TERMINAL_SCROLLBACK_MAX}
        step={100}
        width="w-24"
      />

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
