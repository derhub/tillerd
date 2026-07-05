import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { query } from "@tillerd/client-bindings";
import React from "react";

import {
  Command as CommandBox,
  CommandEmpty,
  CommandInput,
  CommandItem,
  CommandList,
} from "~/components/ui/command";
import { SESSION_SEARCH_OPEN_EVENT } from "~/lib/commands/sessionSearch";
import { sessionDisplayName } from "~/lib/panelTitle";
import { useWindowEvent } from "~/lib/useWindowEvent";

export function SessionSearchDialog() {
  const [open, setOpen] = React.useState(false);
  const [term, setTerm] = React.useState("");
  const navigate = useNavigate();

  useWindowEvent(SESSION_SEARCH_OPEN_EVENT, () => {
    setTerm("");
    setOpen(true);
  });

  const trimmed = term.trim();
  const { data: results = [] } = useQuery({
    ...query("sessionSearch", { query: trimmed }),
    enabled: trimmed.length > 0,
  });

  if (!open) return null;

  const go = (id: string) => {
    setOpen(false);
    void navigate({ to: `/session/${id}` } as never);
  };

  return (
    <div
      className="fixed inset-0 z-[100] flex items-start justify-center bg-black/40 pt-[12vh] animate-in fade-in-0"
      onMouseDown={() => setOpen(false)}
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.stopPropagation();
          setOpen(false);
        }
      }}
      data-testid="session-search"
    >
      <div
        className="w-full max-w-lg overflow-hidden rounded-md border border-border/60 bg-popover shadow-lg"
        onMouseDown={(e) => e.stopPropagation()}
      >
        {/* shouldFilter=false: results are pre-filtered by session_search; cmdk must not re-filter. */}
        <CommandBox loop shouldFilter={false}>
          <CommandInput
            autoFocus
            value={term}
            onValueChange={setTerm}
            placeholder="Search sessions…"
            data-testid="session-search-input"
          />
          <CommandList>
            <CommandEmpty>{trimmed ? "No sessions" : "Type to search sessions"}</CommandEmpty>
            {results.map((s) => (
              <CommandItem key={s.id} value={s.id} onSelect={() => go(s.id)}>
                <span className="truncate">{sessionDisplayName(s.title, s.id)}</span>
              </CommandItem>
            ))}
          </CommandList>
        </CommandBox>
      </div>
    </div>
  );
}
