import type { FileDiffMetadata } from "@pierre/diffs/react";

import DiffsWorker from "@pierre/diffs/worker/worker.js?worker";
import { useQuery } from "@tanstack/react-query";
import { Columns2, AlignJustify } from "lucide-react";
import React from "react";

import { Skeleton } from "~/components/ui/skeleton";
import { lazyDiffsReact } from "~/lib/lazy";
import { API_BASE } from "~/lib/serverUrl";
import { SessionContext } from "~/lib/sessionContext";
import { cn } from "~/lib/utils";

type ViewMode = "stacked" | "split";

async function fetchSessionDiff(sessionId: string): Promise<FileDiffMetadata[]> {
  const r = await fetch(`${API_BASE}/api/sessions/${sessionId}/diff`);
  const patch = await r.text();
  if (!patch.trim()) return [];
  const { parsePatchFiles } = await import("@pierre/diffs");
  const parsed = parsePatchFiles(patch) as Array<{ files: FileDiffMetadata[] }>;
  return parsed.flatMap((p) => p.files);
}

export function DiffPanel({ sessionId }: { sessionId: string | null }) {
  const { status } = React.use(SessionContext);
  const [viewMode, setViewMode] = React.useState<ViewMode>("stacked");

  // Defer until the session settles -- a running session has no final diff yet.
  const enabled = !!sessionId && (status === "IDLE" || status === "DONE");
  const {
    data: files,
    isLoading,
    isError,
    error,
  } = useQuery({
    queryKey: ["diff", sessionId],
    queryFn: () => fetchSessionDiff(sessionId as string),
    enabled,
  });

  if (!sessionId) return <DiffPlaceholder message="No active session" />;
  if (!enabled) return <DiffPlaceholder message="Waiting for session to complete..." />;
  if (isError)
    return (
      <DiffPlaceholder message={error instanceof Error ? error.message : "fetch failed"} error />
    );
  if (isLoading || !files) {
    return (
      <div className="flex flex-col gap-2 p-3">
        <Skeleton className="h-4 w-3/4" />
        <Skeleton className="h-4 w-1/2" />
        <Skeleton className="h-20 w-full" />
      </div>
    );
  }
  if (files.length === 0) return <DiffPlaceholder message="No changes detected" />;

  return (
    <div className="flex flex-col h-full">
      <div
        className="flex items-center gap-1 px-2 shrink-0 border-b border-border"
        style={{ height: "var(--toolbar-height, 2.333rem)" }}
      >
        <span className="text-[0.917rem] text-muted-foreground flex-1">
          {files.length} file{files.length !== 1 ? "s" : ""}
        </span>
        <button
          type="button"
          onClick={() => setViewMode(viewMode === "stacked" ? "split" : "stacked")}
          title={viewMode === "stacked" ? "Switch to split view" : "Switch to stacked view"}
          className={cn(
            "flex items-center justify-center w-5 h-5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
            "text-muted-foreground hover:text-foreground hover:bg-muted",
          )}
        >
          {viewMode === "stacked" ? <Columns2 size={12} /> : <AlignJustify size={12} />}
        </button>
      </div>
      <div className="flex-1 min-h-0 overflow-hidden">
        <DiffView files={files} viewMode={viewMode} />
      </div>
    </div>
  );
}

function DiffPlaceholder({ message, error }: { message: string; error?: boolean }) {
  return (
    <div className="flex h-full items-center justify-center p-4">
      <p
        className={
          error ? "text-[0.917rem] text-destructive" : "text-[0.917rem] text-muted-foreground"
        }
      >
        {message}
      </p>
    </div>
  );
}

type RendererComponent = React.ComponentType<{ files: FileDiffMetadata[]; viewMode: ViewMode }>;

function DiffView({ files, viewMode }: { files: FileDiffMetadata[]; viewMode: ViewMode }) {
  const rendererRef = React.useRef<RendererComponent | null>(null);
  const [Renderer, setRenderer] = React.useState<RendererComponent | null>(null);
  const poolSize = React.useMemo(
    () => Math.max(2, Math.min(6, Math.floor((navigator.hardwareConcurrency || 4) / 2))),
    [],
  );

  React.useEffect(() => {
    if (rendererRef.current) {
      setRenderer(() => rendererRef.current);
      return;
    }
    void loadDiffRenderer(poolSize, rendererRef, (comp) => setRenderer(() => comp));
  }, [poolSize]);

  if (!Renderer) return null;
  return <Renderer files={files} viewMode={viewMode} />;
}

async function loadDiffRenderer(
  poolSize: number,
  rendererRef: React.RefObject<RendererComponent | null>,
  setRenderer: (r: RendererComponent) => void,
): Promise<void> {
  const { FileDiff, Virtualizer, WorkerPoolContextProvider } = await lazyDiffsReact();
  const comp = makeDiffRenderer(FileDiff, Virtualizer, WorkerPoolContextProvider, poolSize);
  rendererRef.current = comp;
  setRenderer(comp);
}

function makeDiffRenderer(
  FileDiff: React.ComponentType<{ fileDiff: FileDiffMetadata; options?: Record<string, unknown> }>,
  Virtualizer: React.ComponentType<{ children: React.ReactNode; className?: string }>,
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  WorkerPoolContextProvider: React.ComponentType<any>,
  poolSize: number,
): RendererComponent {
  function DiffRenderer({ files, viewMode }: { files: FileDiffMetadata[]; viewMode: ViewMode }) {
    return (
      <WorkerPoolContextProvider
        poolOptions={{
          workerFactory: () => new DiffsWorker(),
          poolSize,
          totalASTLRUCacheSize: 240,
        }}
        highlighterOptions={{
          theme: "github-dark",
          tokenizeMaxLineLength: 1_000,
        }}
      >
        <Virtualizer className="h-full overflow-auto px-2 pb-2">
          {files.map((fileDiff, i) => (
            <FileDiff
              key={i}
              fileDiff={fileDiff}
              options={{
                theme: "github-dark",
                overflow: viewMode === "split" ? "scroll" : "wrap",
                lineDiffType: "word",
              }}
            />
          ))}
        </Virtualizer>
      </WorkerPoolContextProvider>
    );
  }
  return DiffRenderer;
}
