import type { LucideIcon } from "lucide-react";
import type { ComponentType } from "react";

import { Command, LayoutTemplate, MessagesSquare, Search } from "lucide-react";

import { SearchView } from "~/components/sidebar/SearchView";
import { ACTION } from "~/lib/commands/ids";

import { ViewPlaceholder } from "./ViewPlaceholder";

// Static registry driving the activity bar and sidebar body, mirroring the command
// model. Order here is the activity-bar order. `Component` is the sidebar body for
// every view except `sessions`, which SidebarContainer renders specially because it
// depends on the window intent (project vs workspace window).
export interface WorkbenchViewDef {
  id: string;
  title: string;
  icon: LucideIcon;
  commandId: string;
  Component: ComponentType;
}

export const VIEW_DEFS: readonly WorkbenchViewDef[] = [
  {
    id: "sessions",
    title: "Sessions",
    icon: MessagesSquare,
    commandId: ACTION.viewSessions,
    // Rendered by SidebarContainer (needs window intent); never invoked here.
    Component: () => null,
  },
  {
    id: "search",
    title: "Search",
    icon: Search,
    commandId: ACTION.viewSearch,
    Component: SearchView,
  },
  {
    id: "commands",
    title: "Commands",
    icon: Command,
    commandId: ACTION.viewCommands,
    Component: () => (
      <ViewPlaceholder icon={Command} title="Commands" hint="Your saved command library." />
    ),
  },
  {
    id: "templates",
    title: "Templates",
    icon: LayoutTemplate,
    commandId: ACTION.viewTemplates,
    Component: () => (
      <ViewPlaceholder
        icon={LayoutTemplate}
        title="Templates"
        hint="Reusable launch templates for new sessions."
      />
    ),
  },
];

export const DEFAULT_VIEW_ID = VIEW_DEFS[0].id;

export function viewDef(id: string): WorkbenchViewDef {
  return VIEW_DEFS.find((v) => v.id === id) ?? VIEW_DEFS[0];
}
