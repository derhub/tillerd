import { useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { query } from "@tillerd/client-bindings";
import { MessagesSquare } from "lucide-react";
import React from "react";

import { UNFILED_ID } from "~/components/sidebar/sidebar-data";
import { Input } from "~/components/ui/input";
import { ScrollArea } from "~/components/ui/scroll-area";
import { setActiveProject, useActiveWorkspace } from "~/lib/store";
import { cn } from "~/lib/utils";

const SEARCH_LIMIT = 20;

// Search activity-bar view: projects + sessions across the active workspace,
// grouped, navigating on activation. The palette session-search (Cmd-P style
// dialog) stays; this is the resident, workspace-scoped surface.
export function SearchView() {
  const workspaceId = useActiveWorkspace() ?? "";
  const navigate = useNavigate();
  const [term, setTerm] = React.useState("");
  const trimmed = term.trim();
  const enabled = trimmed.length > 0 && workspaceId.length > 0;

  // Workspace project set: names the session groups and scopes session hits to
  // this workspace (session_search is global).
  const { data: projects = [] } = useQuery(query("projectList", { workspaceId: workspaceId || null }));
  const projectName = React.useMemo(() => {
    const m = new Map(projects.map((p) => [p.id, p.name] as const));
    m.set(UNFILED_ID, "Unfiled");
    return m;
  }, [projects]);
  const inWorkspace = React.useMemo(() => new Set(projects.map((p) => p.id)), [projects]);

  const { data: projectHits = [] } = useQuery({
    ...query("projectSearch", { workspaceId, query: trimmed, limit: SEARCH_LIMIT }),
    enabled,
  });
  const { data: sessionHits = [] } = useQuery({
    ...query("sessionSearch", { query: trimmed }),
    enabled,
  });

  const scopedSessions = sessionHits.filter((s) => inWorkspace.has(s.projectId));
  const groupedSessions = React.useMemo(() => {
    const groups = new Map<string, typeof scopedSessions>();
    for (const s of scopedSessions) {
      const list = groups.get(s.projectId) ?? [];
      list.push(s);
      groups.set(s.projectId, list);
    }
    return [...groups.entries()];
  }, [scopedSessions]);

  const goProject = (id: string) => {
    setActiveProject(id);
    void navigate({ to: "/" } as never);
  };
  const goSession = (id: string) => void navigate({ to: `/session/${id}` } as never);

  const hasResults = projectHits.length > 0 || scopedSessions.length > 0;

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-8 shrink-0 items-center px-3">
        <span className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground/70">
          Search
        </span>
      </div>
      <div className="px-3 pb-2 shrink-0">
        <Input
          autoFocus
          value={term}
          onChange={(e) => setTerm(e.target.value)}
          // Escape clears this resident search (innermost-overlay rule: it has no
          // dismiss chrome, so Escape empties it in place).
          onKeyDown={(e) => {
            if (e.key === "Escape" && term) {
              e.preventDefault();
              setTerm("");
            }
          }}
          placeholder="Search projects and sessions…"
          data-testid="search-view-input"
          className="h-7 text-[0.833rem]"
        />
      </div>
      <ScrollArea className="flex-1 min-h-0">
        {!enabled ? (
          <p className="px-3 py-3 text-[0.833rem] text-muted-foreground/50 italic">
            Type to search this workspace
          </p>
        ) : !hasResults ? (
          <p
            className="px-3 py-3 text-[0.833rem] text-muted-foreground/50 italic"
            data-testid="search-empty"
          >
            No matches
          </p>
        ) : (
          <div className="flex flex-col gap-3 py-1">
            {projectHits.length > 0 && (
              <div className="flex flex-col gap-px">
                <GroupHeading>Projects</GroupHeading>
                {projectHits.map((p) => (
                  <ResultRow
                    key={p.id}
                    testid="search-project-result"
                    label={p.name}
                    onClick={() => goProject(p.id)}
                  />
                ))}
              </div>
            )}
            {groupedSessions.map(([projectId, list]) => (
              <div key={projectId} className="flex flex-col gap-px">
                <GroupHeading>{projectName.get(projectId) ?? "Sessions"}</GroupHeading>
                {list.map((s) => (
                  <ResultRow
                    key={s.id}
                    testid="search-session-result"
                    icon
                    label={s.title || s.id.slice(0, 8)}
                    onClick={() => goSession(s.id)}
                  />
                ))}
              </div>
            ))}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

function GroupHeading({ children }: { children: React.ReactNode }) {
  return (
    <span className="px-3 text-[0.75rem] font-medium uppercase tracking-wider text-muted-foreground/70 truncate">
      {children}
    </span>
  );
}

function ResultRow({
  label,
  onClick,
  icon,
  testid,
}: {
  label: string;
  onClick: () => void;
  icon?: boolean;
  testid: string;
}) {
  return (
    <button
      type="button"
      data-testid={testid}
      onClick={onClick}
      className={cn(
        "flex items-center gap-2 px-3 h-7 rounded-sm text-left text-[0.833rem] truncate transition-colors duration-[var(--motion-fast)] ease-standard",
        "text-muted-foreground hover:text-foreground hover:bg-muted/50",
      )}
    >
      {icon && (
        <MessagesSquare strokeWidth={2} className="shrink-0 opacity-60 size-[var(--icon-sm)]" />
      )}
      <span className="truncate">{label}</span>
    </button>
  );
}
