import { CaseSensitive, ChevronDown, ChevronUp, X } from "lucide-react";
import React from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";

export interface SearchQueryOptions {
  caseSensitive: boolean;
  incremental?: boolean;
}

// The addon-facing boundary: the pane wires these to an xterm SearchAddon. The overlay never
// touches the terminal directly, so its behaviour is exercised against a fake in happy-dom.
export interface TerminalSearchController {
  findNext(query: string, opts: SearchQueryOptions): void;
  findPrevious(query: string, opts: SearchQueryOptions): void;
  clear(): void;
}

export interface TerminalSearchResults {
  resultIndex: number;
  resultCount: number;
}

function positionLabel(query: string, results: TerminalSearchResults | null): string {
  if (query === "") return "";
  if (!results) return "";
  if (results.resultCount === 0) return "No results";
  if (results.resultIndex < 0) return String(results.resultCount);
  return `${results.resultIndex + 1}/${results.resultCount}`;
}

export function TerminalSearchOverlay({
  controller,
  results,
  initialQuery = "",
  onClose,
}: {
  controller: TerminalSearchController;
  results: TerminalSearchResults | null;
  initialQuery?: string;
  onClose: () => void;
}) {
  const [query, setQuery] = React.useState(initialQuery);
  const [caseSensitive, setCaseSensitive] = React.useState(false);
  const inputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  // Incremental search as the query or case flag changes; an empty query clears the highlights.
  React.useEffect(() => {
    if (query === "") {
      controller.clear();
      return;
    }
    controller.findNext(query, { caseSensitive, incremental: true });
  }, [query, caseSensitive, controller]);

  const findNext = React.useCallback(() => {
    if (query !== "") controller.findNext(query, { caseSensitive });
  }, [controller, query, caseSensitive]);

  const findPrevious = React.useCallback(() => {
    if (query !== "") controller.findPrevious(query, { caseSensitive });
  }, [controller, query, caseSensitive]);

  const onKeyDown = React.useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        if (e.shiftKey) findPrevious();
        else findNext();
      }
    },
    [onClose, findNext, findPrevious],
  );

  return (
    <div
      data-testid="terminal-search"
      role="search"
      className="absolute top-2 right-3 z-10 flex items-center gap-1 border border-terminal-border bg-terminal-surface px-1.5 py-1 text-terminal-fg shadow-md"
    >
      <input
        ref={inputRef}
        data-testid="terminal-search-input"
        aria-label="Find in terminal"
        value={query}
        placeholder="Find"
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={onKeyDown}
        className="h-6 w-40 bg-transparent px-1 text-[0.917rem] outline-none placeholder:text-terminal-muted focus-visible:ring-1 focus-visible:ring-terminal-muted"
      />
      <span
        data-testid="terminal-search-count"
        className="min-w-[3.5rem] text-right text-[0.833rem] text-terminal-muted tabular-nums"
      >
        {positionLabel(query, results)}
      </span>
      <Tooltip>
        <TooltipTrigger
          type="button"
          data-testid="terminal-search-case"
          aria-label="Match case"
          aria-pressed={caseSensitive}
          onClick={() => setCaseSensitive((v) => !v)}
          className={`grid size-6 place-items-center transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-terminal-muted ${
            caseSensitive
              ? "bg-terminal-success/20 text-terminal-fg"
              : "text-terminal-muted hover:text-terminal-fg"
          }`}
        >
          <CaseSensitive className="size-4" />
        </TooltipTrigger>
        <TooltipContent>Match case</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger
          type="button"
          data-testid="terminal-search-prev"
          aria-label="Previous match"
          onClick={findPrevious}
          className="grid size-6 place-items-center text-terminal-muted hover:text-terminal-fg focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-terminal-muted"
        >
          <ChevronUp className="size-4" />
        </TooltipTrigger>
        <TooltipContent>Previous match</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger
          type="button"
          data-testid="terminal-search-next"
          aria-label="Next match"
          onClick={findNext}
          className="grid size-6 place-items-center text-terminal-muted hover:text-terminal-fg focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-terminal-muted"
        >
          <ChevronDown className="size-4" />
        </TooltipTrigger>
        <TooltipContent>Next match</TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger
          type="button"
          data-testid="terminal-search-close"
          aria-label="Close search"
          onClick={onClose}
          className="grid size-6 place-items-center text-terminal-muted hover:text-terminal-fg focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-terminal-muted"
        >
          <X className="size-4" />
        </TooltipTrigger>
        <TooltipContent>Close search</TooltipContent>
      </Tooltip>
    </div>
  );
}
