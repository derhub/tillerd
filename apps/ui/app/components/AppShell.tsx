import { useState, useCallback, useEffect } from "react";
import { useLocation, useNavigate, useParams, useSearchParams } from "react-router";
import { Columns2, Rows2 } from "lucide-react";
import { Panel } from "~/components/Panel";
import { PanelGroup, PanelGroupTabsRoot } from "~/components/PanelGroup";
import { SessionSidebar } from "~/components/SessionSidebar";
import { LogViewer } from "~/components/LogViewer";
import { EmptyPanel } from "~/components/EmptyPanel";
import { TerminalPane } from "~/components/TerminalPane";
import { DesktopTerminalPane } from "~/components/DesktopTerminalPane";
import type { TerminalSurfaceClient } from "@tillerd/sdk/orchestrator";
import { SessionContext } from "~/lib/sessionContext";
import { usePanelTree } from "~/lib/usePanelTree";
import { countLeaves } from "~/lib/panelTree";
import type { PanelNode, PanelGroupNode, PanelLeaf, PanelContent } from "~/lib/panelTree";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { isDesktopHost } from "~/lib/transport";
import { useDelayedTrue } from "~/lib/useDelayedTrue";
import { bootContent } from "~/lib/health/boot-content";
import { ServiceHealthIndicator } from "~/components/ServiceHealthIndicator";
import { SettingsPanel } from "~/components/SettingsPanel";
import { SettingsProvider } from "~/lib/settings/context";
import { Skeleton } from "~/components/ui/skeleton";

// Memoized so spawn and close share one transport instead of re-importing + rebuilding per action.
let terminalClient: Promise<TerminalSurfaceClient> | null = null;
function getTerminalClient(): Promise<TerminalSurfaceClient> {
  return (terminalClient ??= (async () => {
    const { loadTerminalSurfaceTransport } = await import("~/lib/transport/terminal-surface");
    const { createTerminalSurfaceClient } = await import("@tillerd/sdk/orchestrator");
    return createTerminalSurfaceClient(await loadTerminalSurfaceTransport());
  })());
}

export function AppShell() {
  const params = useParams();
  const sessionId = params["id"] ?? null;
  const onLogs = useLocation().pathname === "/logs";
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const logsService = searchParams.get("service") ?? undefined;
  const [status, setStatus] = useState("");

  // Native menu (View > Logs) routes here by emitting "menu:navigate".
  useEffect(() => {
    if (!isDesktopHost()) return;
    let unlisten: (() => void) | undefined;
    void (async () => {
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<string>("menu:navigate", (e) => void navigate(e.payload));
    })();
    return () => unlisten?.();
  }, [navigate]);
  const host = useDesktopHost();
  const orchestratorClient = host.status === "ready" ? host.orchestratorClient : null;
  // Daemon-dependent content (the panels) waits on boot; a skeleton shows only past
  // a short grace so a fast boot never flashes one. The sidebar reads the store and
  // renders immediately regardless.
  const graceElapsed = useDelayedTrue(host.status === "booting", 200);
  const bootRegion = bootContent(host.status, graceElapsed);
  const { tree, split, close, setContent, setActiveTab } = usePanelTree(
    sessionId,
    orchestratorClient,
  );
  const totalPanels = countLeaves(tree);

  const handleSpawn = useCallback(
    async (leafId: string) => {
      if (!sessionId) return;
      const client = await getTerminalClient();
      const placement = await client.spawn(sessionId);
      setContent(leafId, { type: "terminal", placement });
    },
    [sessionId, setContent],
  );

  const handleClose = useCallback(
    (leaf: PanelLeaf) => {
      if (leaf.content.type === "terminal" && sessionId) {
        const placement = leaf.content.placement;
        void getTerminalClient()
          .then((c) => c.close(sessionId, placement))
          .catch(() => {});
      }
      close(leaf.id);
    },
    [sessionId, close],
  );

  const renderNode = useCallback(
    (node: PanelNode, path: string): React.ReactNode => {
      if (node.kind === "group") {
        return renderGroup(node, path);
      }
      return renderLeaf(node, path);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [split, handleClose, handleSpawn, setActiveTab, totalPanels, sessionId, orchestratorClient],
  );

  function renderGroup(group: PanelGroupNode, path: string): React.ReactNode {
    const { displayMode } = group;
    const activeId = group.activeTabId ?? group.children[0]?.id;

    if (displayMode === "split") {
      return (
        <PanelGroup.Provider
          key={group.id}
          id={group.id}
          displayMode={displayMode}
          direction={group.direction}
          activeTabId={activeId}
          onSetActiveTab={(tabId) => setActiveTab(group.id, tabId)}
        >
          <PanelGroup.Split className="h-full">
            {group.children.map((child, i) => (
              <PanelGroup.SplitItem
                key={child.id}
                minSize={10}
                isLast={i === group.children.length - 1}
              >
                {renderNode(child, `${path}-${i}`)}
              </PanelGroup.SplitItem>
            ))}
          </PanelGroup.Split>
        </PanelGroup.Provider>
      );
    }

    return (
      <PanelGroup.Provider
        key={group.id}
        id={group.id}
        displayMode={displayMode}
        direction={group.direction}
        activeTabId={activeId}
        onSetActiveTab={(tabId) => setActiveTab(group.id, tabId)}
      >
        <PanelGroupTabsRoot
          value={activeId}
          onValueChange={(tabId) => setActiveTab(group.id, tabId)}
          className="flex flex-col h-full"
        >
          {displayMode === "tabbar-top" && (
            <PanelGroup.TabBar>
              {group.children.map((child) => (
                <PanelGroup.TabBar.Tab
                  key={child.id}
                  panelId={child.id}
                  title={child.kind === "panel" ? child.title : "Group"}
                />
              ))}
            </PanelGroup.TabBar>
          )}
          <PanelGroup.TabPanels>
            {group.children.map((child) => (
              <PanelGroup.TabContent key={child.id} panelId={child.id}>
                {renderNode(child, `${path}-${child.id}`)}
              </PanelGroup.TabContent>
            ))}
          </PanelGroup.TabPanels>
          {displayMode === "tabbar-bottom" && (
            <PanelGroup.TabBar>
              {group.children.map((child) => (
                <PanelGroup.TabBar.Tab
                  key={child.id}
                  panelId={child.id}
                  title={child.kind === "panel" ? child.title : "Group"}
                />
              ))}
            </PanelGroup.TabBar>
          )}
        </PanelGroupTabsRoot>
      </PanelGroup.Provider>
    );
  }

  function renderLeaf(leaf: PanelLeaf, _path: string): React.ReactNode {
    const title = leaf.content.type === "terminal" ? "Terminal" : "Empty";
    const actions = {
      split: (direction: "horizontal" | "vertical") => split(leaf.id, direction),
      close: () => handleClose(leaf),
    };

    return (
      <Panel.Provider key={leaf.id} id={leaf.id} title={title} actions={actions}>
        <Panel.Frame>
          <Panel.Header>
            <Panel.Title />
            <Panel.Toolbar>
              <Panel.Toolbar.Button
                icon={<Columns2 size={12} />}
                label="Split right"
                onClick={() => split(leaf.id, "horizontal")}
              />
              <Panel.Toolbar.Button
                icon={<Rows2 size={12} />}
                label="Split down"
                onClick={() => split(leaf.id, "vertical")}
              />
              <Panel.CloseButton totalPanels={totalPanels} />
            </Panel.Toolbar>
          </Panel.Header>
          <Panel.Content>{renderContent(leaf.content, leaf.id)}</Panel.Content>
        </Panel.Frame>
      </Panel.Provider>
    );
  }

  function renderContent(content: PanelContent, leafId: string): React.ReactNode {
    switch (content.type) {
      case "empty":
        return (
          <EmptyPanel
            onSpawn={() => void handleSpawn(leafId)}
            disabled={!sessionId || !orchestratorClient}
          />
        );
      case "terminal":
        return (
          <DesktopTerminalPane
            key={`${sessionId ?? "none"}:${content.placement}`}
            sessionId={sessionId}
            placement={content.placement}
            cwd=""
          />
        );
    }
  }

  return (
    <SettingsProvider>
      <SessionContext value={{ sessionId, status, setStatus }}>
        <div className="h-dvh w-full flex overflow-hidden">
          <aside className="w-56 shrink-0 overflow-hidden border-r border-border/40">
            <SessionSidebar />
          </aside>
          <div className="flex-1 min-w-0 pt-px relative">
            {onLogs ? (
              <LogViewer initialService={logsService} />
            ) : host.status === "web" ? (
              <TerminalPane sessionId={sessionId} />
            ) : bootRegion === "content" ? (
              renderNode(tree, "root")
            ) : bootRegion === "skeleton" ? (
              <ContentSkeleton />
            ) : null}
            <div className="fixed bottom-2 right-2 z-50 flex items-center gap-2">
              <SettingsPanel />
              <ServiceHealthIndicator />
            </div>
          </div>
        </div>
      </SessionContext>
    </SettingsProvider>
  );
}

/** Delayed skeleton for the daemon-dependent content region during a slow boot. */
function ContentSkeleton() {
  return (
    <div className="h-full w-full p-3" data-testid="content-skeleton">
      <Skeleton className="h-full w-full" />
    </div>
  );
}
