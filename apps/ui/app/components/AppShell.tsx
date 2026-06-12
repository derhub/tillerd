import { useState, useCallback } from "react";
import { useParams } from "react-router";
import { Outlet } from "react-router";
import { Columns2, Rows2 } from "lucide-react";
import { Panel } from "~/components/Panel";
import { PanelGroup, PanelGroupTabsRoot } from "~/components/PanelGroup";
import { SessionSidebar } from "~/components/SessionSidebar";
import { DiffPanel } from "~/components/DiffPanel";
import { EmptyPanel } from "~/components/EmptyPanel";
import { SessionContext } from "~/lib/sessionContext";
import { usePanelTree } from "~/lib/usePanelTree";
import { countLeaves } from "~/lib/panelTree";
import type { PanelNode, PanelGroupNode, PanelLeaf, PanelContent } from "~/lib/panelTree";
import { useDesktopHost } from "~/lib/useDesktopHost";
import { cn } from "~/lib/utils";

type Session = { id: string; cwd?: string };

type AppShellProps = {
  sessions: Session[];
};

export function AppShell({ sessions }: AppShellProps) {
  const params = useParams();
  const sessionId = params["id"] ?? null;
  const [status, setStatus] = useState("");
  const host = useDesktopHost();
  const orchestratorClient = host.status === "ready" ? host.orchestratorClient : null;
  const { tree, split, close, setContent, setActiveTab } = usePanelTree(
    sessionId,
    orchestratorClient,
  );
  const totalPanels = countLeaves(tree);

  const renderNode = useCallback(
    (node: PanelNode, path: string): React.ReactNode => {
      if (node.kind === "group") {
        return renderGroup(node, path);
      }
      return renderLeaf(node, path);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [split, close, setContent, setActiveTab, sessions, totalPanels],
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

    if (displayMode === "tabbar-top" || displayMode === "tabbar-bottom") {
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

    // sidebar mode
    return (
      <PanelGroup.Provider
        key={group.id}
        id={group.id}
        displayMode={displayMode}
        direction={group.direction}
        activeTabId={activeId}
        onSetActiveTab={(tabId) => setActiveTab(group.id, tabId)}
      >
        <PanelGroup.Sidebar className="h-full">
          {group.children.map((child) => (
            <PanelGroup.Sidebar.Item
              key={child.id}
              panelId={child.id}
              title={child.kind === "panel" ? child.title : "Group"}
            >
              {renderNode(child, `${path}-${child.id}`)}
            </PanelGroup.Sidebar.Item>
          ))}
        </PanelGroup.Sidebar>
      </PanelGroup.Provider>
    );
  }

  function renderLeaf(leaf: PanelLeaf, _path: string): React.ReactNode {
    const actions = {
      split: (direction: "horizontal" | "vertical") => split(leaf.id, direction),
      close: () => close(leaf.id),
    };
    const isEmpty = leaf.content.type === "empty";
    const isTerminal = leaf.content.type === "terminal";
    const hasHeader = !isTerminal && !isEmpty;

    return (
      <Panel.Provider key={leaf.id} id={leaf.id} title={leaf.title} actions={actions}>
        <Panel.Frame>
          {hasHeader && (
            <Panel.Header>
              <Panel.Title />
              <Panel.Toolbar>
                {!isEmpty && (
                  <>
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
                  </>
                )}
                <Panel.CloseButton totalPanels={totalPanels} />
              </Panel.Toolbar>
            </Panel.Header>
          )}
          <Panel.Content>{renderContent(leaf.content, leaf.id)}</Panel.Content>
        </Panel.Frame>
      </Panel.Provider>
    );
  }

  function renderContent(content: PanelContent, panelId: string): React.ReactNode {
    switch (content.type) {
      case "sidebar":
        return <SessionSidebar />;
      case "terminal":
        return <Outlet />;
      case "diff":
        return <DiffPanel sessionId={sessionId} />;
      case "empty":
        return <EmptyPanel onSelect={(c) => setContent(panelId, c)} />;
    }
  }

  return (
    <SessionContext value={{ sessionId, status, setStatus }}>
      <div className="h-dvh w-full overflow-hidden pt-px">
        {renderNode(tree, "root")}
        <HostStatusBadge />
      </div>
    </SessionContext>
  );
}

function HostStatusBadge() {
  const host = useDesktopHost();
  if (host.status === "web") return null;
  const style = {
    booting: { dot: "bg-amber-500", text: "text-amber-300", label: "booting" },
    ready: { dot: "bg-emerald-500", text: "text-emerald-300", label: "ready" },
    error: { dot: "bg-red-500", text: "text-red-300", label: "failed" },
  }[host.status];
  return (
    <div className="fixed bottom-2 right-2 z-50 flex items-center gap-1.5 rounded-sm bg-black/60 px-2 h-6 font-mono text-[0.75rem] pointer-events-none select-none">
      <span className={cn("w-1.5 h-1.5 rounded-full", style.dot)} />
      <span className={style.text}>orchestrator: {style.label}</span>
      {host.status === "error" && (
        <span className="text-red-300/70 max-w-[40ch] truncate">— {host.error.message}</span>
      )}
    </div>
  );
}
