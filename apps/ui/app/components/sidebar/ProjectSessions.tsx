import { useInfiniteQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";

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
  onDeleteSession,
  onReorderSessions,
  onArchiveSession,
}: {
  projectId: string;
  isDesktop: boolean;
  editingId: string | null;
  onStartEditSession: (sessionId: string) => void;
  onCancelEdit: () => void;
  onRenameSession: (sessionId: string, newName: string) => void;
  onDeleteSession: (sessionId: string, name: string) => void;
  onReorderSessions: (orderedIds: string[]) => void;
  onArchiveSession: (id: string, currentPath: string) => void;
}) {
  const { data, fetchNextPage, hasNextPage, isFetchingNextPage, isPending } = useInfiniteQuery(
    query.infinite("sessionList", { projectId, limit: null, offset: null }),
  );
  const sessions = data?.pages.flat() ?? [];

  const handleSessionDrop = (sourceId: string, targetId: string) => {
    const ids = sessions.map((s) => s.id);
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
    <div className="flex flex-col gap-px">
      {sessions.map((s) => (
        <SessionRow
          key={s.id}
          session={s}
          isDesktop={isDesktop}
          isEditing={editingId === s.id}
          onStartEdit={() => onStartEditSession(s.id)}
          onCancelEdit={onCancelEdit}
          onRename={(newName) => onRenameSession(s.id, newName)}
          onArchive={() => onArchiveSession(s.id, window.location.pathname)}
          onDelete={() => onDeleteSession(s.id, s.title || s.id.slice(0, 8))}
          onSessionDrop={handleSessionDrop}
        />
      ))}
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
