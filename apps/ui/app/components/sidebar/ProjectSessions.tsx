import { useInfiniteQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";

import { ArchivedRow, ArchivedSection } from "~/components/sidebar/ArchivedSection";
import type { DeleteTarget } from "~/components/sidebar/DeleteDialog";
import { SessionRow } from "~/components/sidebar/SessionRow";
import { reorderByDrop } from "~/lib/reorder";

// Mounted only while the parent row is expanded; a collapsed project fetches nothing.
// Drag-reorder is limited to loaded sessions (no cross-page reorder).
export function ProjectSessions({
  projectId,
  isDesktop,
  editingId,
  onStartEditSession,
  onCancelEdit,
  onRenameSession,
  onReorderSessions,
  onArchiveSession,
  onRestoreSession,
  onRequestDelete,
}: {
  projectId: string;
  isDesktop: boolean;
  editingId: string | null;
  onStartEditSession: (sessionId: string) => void;
  onCancelEdit: () => void;
  onRenameSession: (sessionId: string, newName: string) => void;
  onReorderSessions: (orderedIds: string[]) => void;
  onArchiveSession: (id: string) => void;
  onRestoreSession: (id: string) => void;
  onRequestDelete: (target: DeleteTarget) => void;
}) {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isPending } = useInfiniteQuery(
    query.infinite("sessionList", { projectId, limit: null, offset: null }),
  );
  const sessions = data?.pages.flat() ?? [];
  const active = sessions.filter((s) => s.status !== "archived");
  const archived = sessions.filter((s) => s.status === "archived");

  const handleSessionDrop = (sourceId: string, targetId: string) => {
    const ids = active.map((s) => s.id);
    const next = reorderByDrop(ids, sourceId, targetId);
    if (next !== ids) onReorderSessions(next);
  };

  if (isPending) {
    return (
      <p
        className="px-3 py-1 text-[0.833rem] text-muted-foreground/50 italic"
        data-testid="sessions-loading"
      >
        Loading…
      </p>
    );
  }

  return (
    <div role="group" className="flex flex-col gap-px">
      {active.map((s) => (
        <SessionRow
          key={s.id}
          session={s}
          projectId={projectId}
          isDesktop={isDesktop}
          isEditing={editingId === s.id}
          onStartEdit={() => onStartEditSession(s.id)}
          onCancelEdit={onCancelEdit}
          onRename={(newName) => onRenameSession(s.id, newName)}
          onArchive={() => onArchiveSession(s.id)}
          onSessionDrop={handleSessionDrop}
        />
      ))}
      <ArchivedSection count={archived.length} className="mt-0.5">
        {archived.map((s) => (
          <ArchivedRow
            key={s.id}
            name={s.title || s.id.slice(0, 8)}
            onRestore={() => onRestoreSession(s.id)}
            onDelete={() =>
              onRequestDelete({ id: s.id, name: s.title || s.id.slice(0, 8), kind: "session" })
            }
          />
        ))}
      </ArchivedSection>
      {hasNextPage && (
        <button
          type="button"
          onClick={() => fetchNextPage()}
          disabled={isFetchingNextPage}
          data-testid="load-more-sessions"
          className="mx-3 mt-0.5 text-left text-[0.75rem] text-muted-foreground/60 hover:text-foreground disabled:opacity-50"
        >
          {isFetchingNextPage ? "Loading…" : "Load more"}
        </button>
      )}
    </div>
  );
}
