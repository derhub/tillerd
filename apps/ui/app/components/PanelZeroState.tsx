import { useMutation, useQuery } from "@tanstack/react-query";
import { useNavigate } from "@tanstack/react-router";
import { command, query } from "@tillerd/client-bindings";
import { FolderPlus } from "lucide-react";

import { PanelContent } from "~/components/shell/PanelContent";
import { UNFILED_ID } from "~/components/sidebar/sidebar-data";
import { Button } from "~/components/ui/button";
import { setActiveProject, useActiveWorkspace } from "~/lib/store";
import { useDesktopHost } from "~/lib/useDesktopHost";

const newSessionArgs = (projectId: string) => ({
  projectId,
  title: null,
  titleSource: "agent-title",
  templateId: null,
});

// Zero-state predicate (task 4.2): the create-project call-to-action replaces the surface picker
// only when there is nothing to work with anywhere -- no named projects and an empty Unfiled
// bucket. Any session (Unfiled or a real project) keeps the picker. The always-present synthetic
// Unfiled project is not a "project" for this decision, so it is excluded from the count.
export function shouldOfferProjectCreation(
  namedProjectCount: number,
  unfiledSessionCount: number,
): boolean {
  return namedProjectCount === 0 && unfiledSessionCount === 0;
}

// Panel-area zero state: on the index route (no active session) with no projects
// in the active workspace, offer a create-project call-to-action opening the
// existing new-project flow. With any project present, the normal panel renders.
export function PanelZeroState() {
  const isDesktop = useDesktopHost().status === "ready";
  const workspaceId = useActiveWorkspace();
  const navigate = useNavigate();
  const createProject = useMutation(command("projectCreate"));
  const createSession = useMutation(command("sessionCreate"));

  // Shares the sidebar's projectList cache (same key) -- no extra fetch. projectList carries the
  // synthetic Unfiled project row, so named projects are the list minus Unfiled and archived.
  const { data: projects, isPending } = useQuery(
    query("projectList", { workspaceId: workspaceId ?? null }),
  );
  const namedProjects = (projects ?? []).filter(
    (p) => p.id !== UNFILED_ID && p.status !== "archived",
  );
  const noNamedProjects = !isPending && namedProjects.length === 0;

  // Unfiled emptiness only matters with no named project; the fetch is gated off otherwise, and
  // one row (limit 1) settles the decision.
  const { data: unfiledSessions, isPending: unfiledPending } = useQuery({
    ...query("sessionList", { projectId: UNFILED_ID, limit: 1, offset: null }),
    enabled: isDesktop && noNamedProjects,
  });

  const create = () => {
    if (!isDesktop) return;
    const name = window.prompt("Project name (leave blank for a blank project):") ?? "";
    createProject.mutate(
      { name: name.trim() || null, workspaceId: workspaceId ?? null },
      {
        onSuccess: (proj) => {
          createSession.mutate(newSessionArgs(proj.id), {
            onSuccess: (sess) => {
              setActiveProject(proj.id);
              void navigate({ to: `/session/${sess.id}` } as never);
            },
          });
        },
      },
    );
  };

  const offerCreate =
    isDesktop &&
    noNamedProjects &&
    !unfiledPending &&
    shouldOfferProjectCreation(namedProjects.length, unfiledSessions?.length ?? 0);

  if (offerCreate) {
    return (
      <div
        data-testid="panel-create-project"
        className="flex h-full w-full flex-col items-center justify-center gap-3 p-6 text-center"
      >
        <FolderPlus className="size-[var(--icon-lg)] text-muted-foreground/40" aria-hidden />
        <div className="flex flex-col gap-1">
          <p className="text-sm font-medium">No projects yet</p>
          <p className="text-[0.833rem] text-muted-foreground/60">
            Create a project to start a session.
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={create}>
          New project
        </Button>
      </div>
    );
  }
  return <PanelContent />;
}
