import React from "react";

import type { ContextValue } from "~/lib/commands/when";

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "~/components/ui/context-menu";
import { setContextKey } from "~/lib/commands/context";
import { useSurfaceCommands } from "~/lib/commands/registry";

export interface EntityContextMenuProps extends React.ComponentProps<"div"> {
  // Identifies the row an invoked command acts on -- passed as `run({ entityId,
  // entityKind, ...args })` so a single centrally-registered handler can act on
  // whichever row the user actually right-clicked (no per-row registration; the
  // registry only keeps one handler per command id).
  entityId: string;
  entityKind: string;
  // Extra invocation-time payload a handler needs beyond identity (e.g. the
  // row's display name for a delete confirmation) -- merged into the args.
  args?: Record<string, unknown>;
  // Extra `when` guards scoped to this specific row (e.g. "menu.canDelete" for
  // a project that fails the domain guard) -- pushed alongside the entityKind's
  // row-scope flag while the menu is open and cleared on close. See context.ts.
  guards?: Record<string, ContextValue>;
  // Skips the context-menu wrapper entirely (e.g. non-desktop hosts, which never
  // wired a right-click menu).
  disabled?: boolean;
}

// Generic row context menu: a projection of every `contextmenu`-tagged command
// whose `when` currently passes, scoped to this entity by a `menu.<kind>Row`
// context flag. Adding a new row-scoped command to defs.ts is enough for it to
// appear here -- this component never lists actions by id.
export function EntityContextMenu({
  entityId,
  entityKind,
  args,
  guards,
  disabled,
  children,
  ...triggerProps
}: EntityContextMenuProps) {
  const commands = useSurfaceCommands("contextmenu");

  // Read fresh on each open/close/unmount rather than memoized -- `guards` and
  // `args` are recomputed from live per-row state on every render, so a memo
  // keyed on their identity would never hit anyway.
  const scopeRef = React.useRef<Record<string, ContextValue>>({});
  scopeRef.current = { [`menu.${entityKind}Row`]: true, ...guards };

  const handleOpenChange = React.useCallback((open: boolean) => {
    for (const key of Object.keys(scopeRef.current)) {
      setContextKey(key, open ? scopeRef.current[key] : undefined);
    }
  }, []);

  // A row can unmount while its menu is open (project deleted from elsewhere) --
  // clear this instance's flags so a stale scope can't keep gating the menu.
  React.useEffect(() => {
    return () => {
      for (const key of Object.keys(scopeRef.current)) setContextKey(key, undefined);
    };
  }, []);

  if (disabled) {
    return <div {...triggerProps}>{children}</div>;
  }

  const runArgs = { entityId, entityKind, ...args };

  let lastGroup: string | undefined;
  return (
    <ContextMenu onOpenChange={handleOpenChange}>
      <ContextMenuTrigger {...triggerProps}>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        {commands.map((command) => {
          const separator = lastGroup !== undefined && command.group !== lastGroup;
          lastGroup = command.group;
          const Icon = command.icon;
          return (
            <React.Fragment key={command.id}>
              {separator && <ContextMenuSeparator />}
              <ContextMenuItem
                data-testid={`context-menu-${command.id}`}
                variant={command.group === "destructive" ? "destructive" : "default"}
                onClick={() => command.run(runArgs)}
              >
                {Icon && <Icon className="size-[var(--icon-sm)]" />}
                <span>{command.title}</span>
              </ContextMenuItem>
            </React.Fragment>
          );
        })}
      </ContextMenuContent>
    </ContextMenu>
  );
}
