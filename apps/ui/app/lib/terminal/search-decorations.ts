import type { ISearchOptions } from "@xterm/addon-search";

// SearchAddon match highlights require literal #RRGGBB (no CSS token indirection), tuned to read on
// the dark terminal canvas. Kept in lib so the component token-lint stays clean.
const SEARCH_DECORATIONS = {
  matchBackground: "#665c1e",
  activeMatchBackground: "#d29922",
  matchOverviewRuler: "#d29922",
  activeMatchColorOverviewRuler: "#d29922",
};

export function toSearchOptions(opts: {
  caseSensitive: boolean;
  incremental?: boolean;
}): ISearchOptions {
  return {
    caseSensitive: opts.caseSensitive,
    incremental: opts.incremental,
    decorations: SEARCH_DECORATIONS,
  };
}
