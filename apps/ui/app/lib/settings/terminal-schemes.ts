export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

const GITHUB_DARK: TerminalTheme = {
  background: "#0d1117",
  foreground: "#e6edf3",
  cursor: "#e6edf3",
  black: "#0d1117",
  red: "#ff7b72",
  green: "#3fb950",
  yellow: "#d29922",
  blue: "#58a6ff",
  magenta: "#bc8cff",
  cyan: "#39c5cf",
  white: "#b1bac4",
  brightBlack: "#6e7681",
  brightRed: "#ffa198",
  brightGreen: "#56d364",
  brightYellow: "#e3b341",
  brightBlue: "#79c0ff",
  brightMagenta: "#d2a8ff",
  brightCyan: "#56d4dd",
  brightWhite: "#f0f6fc",
};

const GITHUB_LIGHT: TerminalTheme = {
  background: "#ffffff",
  foreground: "#1f2328",
  cursor: "#1f2328",
  black: "#24292f",
  red: "#cf222e",
  green: "#116329",
  yellow: "#4d2d00",
  blue: "#0969da",
  magenta: "#8250df",
  cyan: "#1b7c83",
  white: "#6e7781",
  brightBlack: "#57606a",
  brightRed: "#a40e26",
  brightGreen: "#1a7f37",
  brightYellow: "#633c01",
  brightBlue: "#218bff",
  brightMagenta: "#a475f9",
  brightCyan: "#3192aa",
  brightWhite: "#8c959f",
};

// Terminal canvas is independent of the app theme.
const TERMINAL_SCHEMES: Record<string, TerminalTheme> = {
  "github-dark": GITHUB_DARK,
  "github-light": GITHUB_LIGHT,
};

export const DEFAULT_TERMINAL_SCHEME = "github-dark";

export function getTerminalTheme(name: string): TerminalTheme {
  return TERMINAL_SCHEMES[name] ?? TERMINAL_SCHEMES[DEFAULT_TERMINAL_SCHEME];
}

export const TERMINAL_SCHEME_NAMES: readonly string[] = Object.keys(TERMINAL_SCHEMES);
