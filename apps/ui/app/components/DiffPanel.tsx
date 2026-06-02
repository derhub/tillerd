import { useEffect, useState, use, useRef, useMemo } from "react";
import { Columns2, AlignJustify } from "lucide-react";
import { Skeleton } from "~/components/ui/skeleton";
import { SessionContext } from "~/lib/sessionContext";
import { cn } from "~/lib/utils";
import type { FileDiffMetadata } from "@pierre/diffs/react";
import DiffsWorker from "@pierre/diffs/worker/worker.js?worker";

const API_BASE = `http://${typeof window !== "undefined" ? window.location.hostname : "localhost"}:3000`;

type DiffState =
  | { phase: "idle" }
  | { phase: "loading" }
  | { phase: "done"; files: FileDiffMetadata[] }
  | { phase: "error"; message: string };

type ViewMode = "stacked" | "split";

export function DiffPanel({ sessionId }: { sessionId: string | null }) {
  const { status } = use(SessionContext);
  const [diff, setDiff] = useState<DiffState>({ phase: "idle" });
  const [viewMode, setViewMode] = useState<ViewMode>("stacked");

  useEffect(() => {
    if (!sessionId) return;
    if (status !== "IDLE" && status !== "DONE") return;

    setDiff({ phase: "loading" });
    fetch(`${API_BASE}/api/sessions/${sessionId}/diff`)
      .then(async (r) => {
        const patch = await r.text();
        if (!patch.trim()) {
          setDiff({ phase: "done", files: [] });
          return;
        }
        const { parsePatchFiles } = await import("@pierre/diffs");
        const parsed = parsePatchFiles(patch) as Array<{ files: FileDiffMetadata[] }>;
        const files = parsed.flatMap((p) => p.files);
        setDiff({ phase: "done", files });
      })
      .catch((err: unknown) =>
        setDiff({ phase: "error", message: err instanceof Error ? err.message : "fetch failed" }),
      );
  }, [sessionId, status]);

  if (!sessionId) return <DiffPlaceholder message="No active session" />;
  if (diff.phase === "idle") return <DiffPlaceholder message="Waiting for session to complete…" />;
  if (diff.phase === "error") return <DiffPlaceholder message={diff.message} error />;

  if (diff.phase === "loading") {
    return (
      <div className="flex flex-col gap-2 p-3">
        <Skeleton className="h-4 w-3/4" />
        <Skeleton className="h-4 w-1/2" />
        <Skeleton className="h-20 w-full" />
      </div>
    );
  }

  if (diff.files.length === 0) return <DiffPlaceholder message="No changes detected" />;

  return (
    <div className="flex flex-col h-full">
      <div
        className="flex items-center gap-1 px-2 shrink-0 border-b border-border"
        style={{ height: "var(--toolbar-height, 2.333rem)" }}
      >
        <span className="text-[0.917rem] text-muted-foreground flex-1">
          {diff.files.length} file{diff.files.length !== 1 ? "s" : ""}
        </span>
        <button
          type="button"
          onClick={() => setViewMode(viewMode === "stacked" ? "split" : "stacked")}
          title={viewMode === "stacked" ? "Switch to split view" : "Switch to stacked view"}
          className={cn(
            "flex items-center justify-center w-5 h-5 rounded-sm transition-colors",
            "text-muted-foreground hover:text-foreground hover:bg-muted",
          )}
        >
          {viewMode === "stacked" ? <Columns2 size={12} /> : <AlignJustify size={12} />}
        </button>
      </div>
      <div className="flex-1 min-h-0 overflow-hidden">
        <DiffView files={diff.files} viewMode={viewMode} />
      </div>
    </div>
  );
}

function DiffPlaceholder({ message, error }: { message: string; error?: boolean }) {
  return (
    <div className="flex h-full items-center justify-center p-4">
      <p className={error ? "text-[0.917rem] text-destructive" : "text-[0.917rem] text-muted-foreground"}>
        {message}
      </p>
    </div>
  );
}

type RendererComponent = React.ComponentType<{ files: FileDiffMetadata[]; viewMode: ViewMode }>;

function DiffView({ files, viewMode }: { files: FileDiffMetadata[]; viewMode: ViewMode }) {
  const rendererRef = useRef<RendererComponent | null>(null);
  const [Renderer, setRenderer] = useState<RendererComponent | null>(null);
  const poolSize = useMemo(() => Math.max(2, Math.min(6, Math.floor((navigator.hardwareConcurrency || 4) / 2))), []);

  useEffect(() => {
    if (rendererRef.current) {
      setRenderer(() => rendererRef.current);
      return;
    }
    (async () => {
      const { FileDiff, Virtualizer, WorkerPoolContextProvider } = await import("@pierre/diffs/react");
      const comp = makeDiffRenderer(FileDiff, Virtualizer, WorkerPoolContextProvider, poolSize);
      rendererRef.current = comp;
      setRenderer(() => comp);
    })();
  }, [poolSize]);

  if (!Renderer) return null;
  return <Renderer files={files} viewMode={viewMode} />;
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
