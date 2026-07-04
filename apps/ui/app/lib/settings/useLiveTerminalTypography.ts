import type { Terminal } from "@xterm/xterm";

import React from "react";

import { useBoolGlobalSetting, useGlobalSetting, useNumberGlobalSetting } from "./context";
import {
  DEFAULT_TERMINAL_CONFIRM_PASTE,
  DEFAULT_TERMINAL_COPY_ON_SELECT,
  DEFAULT_TERMINAL_CURSOR_BLINK,
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
  TERMINAL_SCROLLBACK_KEY,
  type TerminalCursorStyle,
} from "./keys";

export interface TerminalTypography {
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  cursorStyle: TerminalCursorStyle;
  cursorBlink: boolean;
  scrollback: number;
}

// Companion to useLiveTerminalTheme for typography and buffer options: reads the settings and,
// on any change, mirrors them onto the mounted terminal's live `options.*` and refits so the PTY
// relearns the new column/row geometry (ui-terminal-pane "typography and buffer settings apply
// live"). termRef may be null before the Terminal exists; the effect no-ops until mount, and the
// returned values seed the caller's construction options for that first paint.
export function useLiveTerminalTypography(
  termRef: React.RefObject<Terminal | null>,
  refit?: () => void,
): TerminalTypography {
  const { value: fontSize } = useNumberGlobalSetting(
    TERMINAL_FONT_SIZE_KEY,
    DEFAULT_TERMINAL_FONT_SIZE,
  );
  const { value: fontFamily } = useGlobalSetting(
    TERMINAL_FONT_FAMILY_KEY,
    DEFAULT_TERMINAL_FONT_FAMILY,
  );
  const { value: lineHeight } = useNumberGlobalSetting(
    TERMINAL_LINE_HEIGHT_KEY,
    DEFAULT_TERMINAL_LINE_HEIGHT,
  );
  const { value: cursorStyleRaw } = useGlobalSetting(
    TERMINAL_CURSOR_STYLE_KEY,
    DEFAULT_TERMINAL_CURSOR_STYLE,
  );
  const { value: cursorBlink } = useBoolGlobalSetting(
    TERMINAL_CURSOR_BLINK_KEY,
    DEFAULT_TERMINAL_CURSOR_BLINK,
  );
  const { value: scrollback } = useNumberGlobalSetting(
    TERMINAL_SCROLLBACK_KEY,
    DEFAULT_TERMINAL_SCROLLBACK,
  );

  const cursorStyle = isTerminalCursorStyle(cursorStyleRaw)
    ? cursorStyleRaw
    : DEFAULT_TERMINAL_CURSOR_STYLE;

  const typography = React.useMemo<TerminalTypography>(
    () => ({ fontSize, fontFamily, lineHeight, cursorStyle, cursorBlink, scrollback }),
    [fontSize, fontFamily, lineHeight, cursorStyle, cursorBlink, scrollback],
  );

  React.useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    term.options.fontSize = typography.fontSize;
    term.options.fontFamily = typography.fontFamily;
    term.options.lineHeight = typography.lineHeight;
    term.options.cursorStyle = typography.cursorStyle;
    term.options.cursorBlink = typography.cursorBlink;
    term.options.scrollback = typography.scrollback;
    refit?.();
  }, [typography, termRef, refit]);

  return typography;
}

// Clipboard-hygiene settings live alongside typography but are consumed by the pane's copy/paste
// wiring rather than pushed onto xterm options, so they get a lightweight companion reader.
export interface TerminalClipboardSettings {
  copyOnSelect: boolean;
  confirmPaste: boolean;
}

export function useTerminalClipboardSettings(): TerminalClipboardSettings {
  const { value: copyOnSelect } = useBoolGlobalSetting(
    TERMINAL_COPY_ON_SELECT_KEY,
    DEFAULT_TERMINAL_COPY_ON_SELECT,
  );
  const { value: confirmPaste } = useBoolGlobalSetting(
    TERMINAL_CONFIRM_PASTE_KEY,
    DEFAULT_TERMINAL_CONFIRM_PASTE,
  );
  return { copyOnSelect, confirmPaste };
}
