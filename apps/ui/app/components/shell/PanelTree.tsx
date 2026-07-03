import { ArrowUpRight, Columns2, ExternalLink, Rows2 } from "lucide-react";

import type { PanelContent, PanelGroupNode, PanelLeaf, PanelNode } from "~/lib/panelTree";

import { EmptyPanel } from "~/components/shell/EmptyPanel";
import { Panel } from "~/components/shell/Panel";
import { PanelGroup, PanelGroupTabsRoot } from "~/components/shell/PanelGroup";
import { DesktopTerminalPane } from "~/components/terminal/DesktopTerminalPane";

export function PanelTree({
  tree,
  totalPanels,
  sessionId,
  detached,
  onSplit,
  onSetActiveTab,
  onClose,
  onSpawn,
  onDetach,
  onReattach,
}: {
  tree: PanelNode;
  totalPanels: number;
  sessionId: string | null;
  detached: Set<string>;
  onSplit: (leafId: string, direction: "horizontal" | "vertical") => void;
  onSetActiveTab: (groupId: string, tabId: string) => void;
  onClose: (leaf: PanelLeaf) => void;
  onSpawn: (leafId: string) => void;
  onDetach: (leaf: PanelLeaf) => void;
  onReattach: (placement: string) => void;
}) {
  function renderNode(node: PanelNode, path: string): React.ReactNode {
    return node.kind === "group" ? renderGroup(node, path) : renderLeaf(node);
  }

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
          onSetActiveTab={(tabId) => onSetActiveTab(group.id, tabId)}
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
        onSetActiveTab={(tabId) => onSetActiveTab(group.id, tabId)}
      >
        <PanelGroupTabsRoot
          value={activeId}
          onValueChange={(tabId) => onSetActiveTab(group.id, tabId)}
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

  function renderLeaf(leaf: PanelLeaf): React.ReactNode {
    const title = leaf.content.type === "terminal" ? "Terminal" : "Empty";
    const actions = {
      split: (direction: "horizontal" | "vertical") => onSplit(leaf.id, direction),
      close: () => onClose(leaf),
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
                onClick={() => onSplit(leaf.id, "horizontal")}
              />
              <Panel.Toolbar.Button
                icon={<Rows2 size={12} />}
                label="Split down"
                onClick={() => onSplit(leaf.id, "vertical")}
              />
              {leaf.content.type === "terminal" && !detached.has(leaf.content.placement) && (
                <Panel.Toolbar.Button
                  icon={<ExternalLink size={12} />}
                  label="Detach"
                  onClick={() => onDetach(leaf)}
                />
              )}
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
        return <EmptyPanel onSpawn={() => onSpawn(leafId)} disabled={!sessionId} />;
      case "terminal":
        if (detached.has(content.placement)) {
          return <DetachedPlaceholder onReattach={() => onReattach(content.placement)} />;
        }
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

  return <>{renderNode(tree, "root")}</>;
}

function DetachedPlaceholder({ onReattach }: { onReattach: () => void }) {
  return (
    <div
      className="flex h-full w-full flex-col items-center justify-center gap-2 bg-muted/20"
      data-testid="detached-placeholder"
    >
      <span className="text-[0.917rem] text-muted-foreground select-none">
        Detached to a separate window
      </span>
      <button
        type="button"
        onClick={onReattach}
        aria-label="Re-attach detached window"
        className="flex items-center gap-1 px-2 h-6 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard"
      >
        <span>Re-attach</span>
        <ArrowUpRight size={12} />
      </button>
    </div>
  );
}
