import type { SpawnCommandRef } from "@tillerd/client-bindings";

import { useQueries, useQuery } from "@tanstack/react-query";
import { query } from "@tillerd/client-bindings";
import { ArrowUpRight, Columns2, ExternalLink, Rows2 } from "lucide-react";
import React from "react";

import { EmptyPanel } from "~/components/shell/EmptyPanel";
import { Panel } from "~/components/shell/Panel";
import { PanelGroup, PanelGroupTabsRoot } from "~/components/shell/PanelGroup";
import { DesktopTerminalPane } from "~/components/terminal/DesktopTerminalPane";
import { commandListQuery } from "~/lib/data/commands";
import { sessionDisplayName, terminalTitle } from "~/lib/panelTitle";
import {
  collectLeaves,
  DRAG_PANEL_LEAF,
  type PanelContent,
  type PanelGroupNode,
  type PanelLeaf,
  type PanelNode,
} from "~/lib/panelTree";
import { useElapsedTick } from "~/lib/useElapsedTick";

export function PanelTree({
  tree,
  totalPanels,
  sessionId,
  sessionTitle,
  detached,
  closingLeafIds,
  reloadEpoch,
  focusedLeafId,
  zoomedLeafId,
  onSplit,
  onSetActiveTab,
  onClose,
  onSpawn,
  onDetach,
  onReattach,
  onSwapPlacements,
  onStatusChange,
  onRequestReset,
  onRestart,
}: {
  tree: PanelNode;
  totalPanels: number;
  sessionId: string | null;
  sessionTitle?: string;
  detached: Set<string>;
  closingLeafIds: Set<string>;
  reloadEpoch: Record<string, number>;
  focusedLeafId: string | null;
  zoomedLeafId: string | null;
  onSplit: (leafId: string, direction: "horizontal" | "vertical") => void;
  onSetActiveTab: (groupId: string, tabId: string) => void;
  onClose: (leaf: PanelLeaf) => void;
  onSpawn: (leafId: string, command?: SpawnCommandRef) => void;
  onDetach: (leaf: PanelLeaf) => void;
  onReattach: (placement: string) => void;
  onSwapPlacements: (sourceLeafId: string, targetLeafId: string) => void;
  onStatusChange: (placement: string, status: string) => void;
  onRequestReset: (leafId: string) => void;
  onRestart: (leafId: string) => void;
}) {
  // Drag/drop is transient UI state, not part of the persisted layout tree
  // (panel-placement-swap spec: geometry is unchanged, only the surface binding swaps).
  const [draggingLeafId, setDraggingLeafId] = React.useState<string | null>(null);
  const [dropTargetLeafId, setDropTargetLeafId] = React.useState<string | null>(null);

  // Elapsed-since-spawn for the panel title (ui-panel-compound spec): one query per terminal
  // leaf's already-bound surface (cached, no polling) plus one coarse 30s tick for this whole
  // panel area to keep the displayed "Xm"/"Xh Ym" text current.
  const terminalPlacements = React.useMemo(
    () =>
      collectLeaves(tree).flatMap((leaf) =>
        leaf.content.type === "terminal" ? [leaf.content.placement] : [],
      ),
    [tree],
  );
  const surfaceQueries = useQueries({
    queries: terminalPlacements.map((placement) => ({
      ...query("surfaceFindByPlacement", { session: sessionId ?? "", placement }),
      enabled: Boolean(sessionId),
    })),
  });
  const spawnedAtByPlacement = new Map<string, number | null>(
    terminalPlacements.map((placement, i) => [
      placement,
      surfaceQueries[i]?.data?.spawnedAt ?? null,
    ]),
  );
  const now = useElapsedTick();

  // Empty panel picker (ui-panel-compound "Empty panel picker", usability pass 12.4): the command
  // library, so an empty leaf can spawn a terminal running a stored command, not just a login
  // shell. Cached by TanStack Query, so multiple empty leaves in the same tree share one fetch.
  // Gated on the tree actually having an empty leaf so fully-populated panels (the common case)
  // never issue the command_list fetch on the hot session-switch path.
  const hasEmptyLeaf = React.useMemo(
    () => collectLeaves(tree).some((leaf) => leaf.content.type === "empty"),
    [tree],
  );
  const { data: emptyPanelCommands = [] } = useQuery({
    ...commandListQuery(),
    enabled: hasEmptyLeaf,
  });

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
    // Title content (ui-panel-compound "Panel title content"): session name + surface kind +
    // elapsed time since the surface's PTY spawned (hidden while spawned_at is null).
    const title =
      leaf.content.type === "terminal"
        ? terminalTitle(
            sessionDisplayName(sessionTitle, sessionId),
            spawnedAtByPlacement.get(leaf.content.placement) ?? null,
            now,
          )
        : "Empty";
    const actions = {
      split: (direction: "horizontal" | "vertical") => onSplit(leaf.id, direction),
      close: () => onClose(leaf),
    };
    const isTerminal = leaf.content.type === "terminal";
    const isDropTarget = dropTargetLeafId === leaf.id;
    // A terminal leaf can always close (it resets to the picker in place); an empty leaf can close
    // only when it is not the sole pane (closing removes it). Sole-empty hides the control.
    const canClose = isTerminal || totalPanels > 1;

    return (
      <Panel.Provider key={leaf.id} id={leaf.id} title={title} actions={actions}>
        <Panel.Frame
          isClosing={closingLeafIds.has(leaf.id)}
          isDropTarget={isDropTarget}
          isFocused={focusedLeafId === leaf.id}
          onDragOver={(e) => {
            if (!isTerminal || draggingLeafId === leaf.id) return;
            if (e.dataTransfer.types.includes(DRAG_PANEL_LEAF)) {
              e.preventDefault();
              e.dataTransfer.dropEffect = "move";
              setDropTargetLeafId(leaf.id);
            }
          }}
          onDragLeave={() => setDropTargetLeafId((id) => (id === leaf.id ? null : id))}
          onDrop={(e) => {
            e.preventDefault();
            setDropTargetLeafId(null);
            const sourceId = e.dataTransfer.getData(DRAG_PANEL_LEAF);
            if (sourceId && sourceId !== leaf.id && isTerminal) onSwapPlacements(sourceId, leaf.id);
          }}
        >
          <Panel.Header
            draggable={isTerminal}
            onDragStart={(e) => {
              e.dataTransfer.setData(DRAG_PANEL_LEAF, leaf.id);
              e.dataTransfer.effectAllowed = "move";
              setDraggingLeafId(leaf.id);
            }}
            onDragEnd={() => {
              setDraggingLeafId(null);
              setDropTargetLeafId(null);
            }}
          >
            <Panel.Title />
            <Panel.Toolbar>
              <Panel.Toolbar.Button
                icon={<Columns2 className="size-[var(--icon-md)]" />}
                label="Split right"
                onClick={() => onSplit(leaf.id, "horizontal")}
              />
              <Panel.Toolbar.Button
                icon={<Rows2 className="size-[var(--icon-md)]" />}
                label="Split down"
                onClick={() => onSplit(leaf.id, "vertical")}
              />
              {leaf.content.type === "terminal" && !detached.has(leaf.content.placement) && (
                <Panel.Toolbar.Button
                  icon={<ExternalLink className="size-[var(--icon-md)]" />}
                  label="Detach"
                  onClick={() => onDetach(leaf)}
                />
              )}
              <Panel.CloseButton canClose={canClose} />
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
            onSpawn={(cmd) => onSpawn(leafId, cmd)}
            disabled={!sessionId}
            commands={emptyPanelCommands}
          />
        );
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
            reloadKey={reloadEpoch[content.placement] ?? 0}
            onStatusChange={onStatusChange}
            onRequestReset={() => onRequestReset(leafId)}
            onRestart={() => onRestart(leafId)}
          />
        );
    }
  }

  // Zoom (panel-multiplexer-nav spec): render only the zoomed leaf, filling the whole panel area.
  // The persisted tree is untouched -- unzoom just clears the transient zoomedLeafId. A stale id
  // (leaf gone) falls back to the normal tree render.
  if (zoomedLeafId) {
    const zoomed = collectLeaves(tree).find((l) => l.id === zoomedLeafId);
    if (zoomed) return renderLeaf(zoomed);
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
        className="flex items-center gap-1 px-2 h-6 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      >
        <span>Re-attach</span>
        <ArrowUpRight className="size-[var(--icon-sm)]" />
      </button>
    </div>
  );
}
