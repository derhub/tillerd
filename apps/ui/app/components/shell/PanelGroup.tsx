import type { GroupImperativeHandle, Layout } from "react-resizable-panels";

import { ChevronRight } from "lucide-react";
import React from "react";

import type { DisplayMode } from "~/lib/panelTree";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "~/components/ui/collapsible";
import { ResizablePanelGroup, ResizablePanel, ResizableHandle } from "~/components/ui/resizable";
import { TabsList, TabsTrigger, Tabs, TabsContent } from "~/components/ui/tabs";
import { cn } from "~/lib/utils";

type PanelGroupContextValue = {
  state: {
    id: string;
    displayMode: DisplayMode;
    activeTabId: string | undefined;
    direction: "horizontal" | "vertical";
  };
  actions: {
    setActiveTab: (tabId: string) => void;
  };
};

const PanelGroupContext = React.createContext<PanelGroupContextValue | null>(null);

const PanelSplitContext = React.createContext<{ reset: () => void } | null>(null);

function panelSizesMatch(actual: readonly number[], expected: readonly number[]): boolean {
  return (
    actual.length === expected.length &&
    actual.every((size, index) => Math.abs(size - expected[index]!) < 0.01)
  );
}

// Object layout keys need a non-numeric prefix to preserve child order.
function resizePanelId(id: string): string {
  return `panel:${id}`;
}

function usePanelGroupContext() {
  const ctx = React.use(PanelGroupContext);
  if (!ctx) throw new Error("PanelGroup sub-components must be used inside PanelGroup.Provider");
  return ctx;
}

function PanelGroupProvider({
  children,
  id,
  displayMode,
  activeTabId,
  direction,
  onSetActiveTab,
}: {
  children: React.ReactNode;
  id: string;
  displayMode: DisplayMode;
  activeTabId: string | undefined;
  direction: "horizontal" | "vertical";
  onSetActiveTab: (tabId: string) => void;
}) {
  return (
    <PanelGroupContext
      value={{
        state: { id, displayMode, activeTabId, direction },
        actions: { setActiveTab: onSetActiveTab },
      }}
    >
      {children}
    </PanelGroupContext>
  );
}

function PanelGroupSplit({
  children,
  childIds,
  sizes,
  onSizesChange,
  className,
}: {
  children: React.ReactNode;
  childIds: string[];
  sizes: number[];
  onSizesChange: (sizes: number[]) => void;
  className?: string;
}) {
  const { state } = usePanelGroupContext();
  const groupRef = React.useRef<GroupImperativeHandle>(null);
  const ignoredLayoutRef = React.useRef<Layout | null>(null);
  const defaultLayout = Object.fromEntries(
    childIds.map((id, index) => [resizePanelId(id), sizes[index] as number]),
  );
  const onLayoutChanged = React.useCallback(
    (layout: Layout) => {
      const nextSizes = childIds.map((id) => layout[resizePanelId(id)] as number);
      const ignored = ignoredLayoutRef.current;
      ignoredLayoutRef.current = null;
      if (
        ignored &&
        panelSizesMatch(
          nextSizes,
          childIds.map((id) => ignored[resizePanelId(id)] as number),
        )
      ) {
        return;
      }
      if (!panelSizesMatch(nextSizes, sizes)) onSizesChange(nextSizes);
    },
    [childIds, onSizesChange, sizes],
  );
  const reset = React.useCallback(() => {
    const equalSizes = childIds.map(() => 100 / childIds.length);
    const equalLayout = Object.fromEntries(
      childIds.map((id, index) => [resizePanelId(id), equalSizes[index] as number]),
    );
    ignoredLayoutRef.current = equalLayout;
    groupRef.current?.setLayout(equalLayout);
    onSizesChange(equalSizes);
  }, [childIds, onSizesChange]);

  return (
    <PanelSplitContext value={{ reset }}>
      <ResizablePanelGroup
        id={state.id}
        groupRef={groupRef}
        orientation={state.direction}
        defaultLayout={defaultLayout}
        onLayoutChanged={onLayoutChanged}
        className={cn("h-full", className)}
      >
        {children}
      </ResizablePanelGroup>
    </PanelSplitContext>
  );
}

function PanelGroupSplitItem({
  children,
  panelId,
  minSize,
  isLast,
}: {
  children: React.ReactNode;
  panelId: string;
  minSize?: number;
  isLast: boolean;
}) {
  const split = React.use(PanelSplitContext);
  if (!split) throw new Error("PanelGroup.SplitItem must be used inside PanelGroup.Split");
  return (
    <>
      <ResizablePanel
        id={resizePanelId(panelId)}
        data-panel-node-id={panelId}
        minSize={`${minSize ?? 0}%`}
      >
        {children}
      </ResizablePanel>
      {!isLast && <ResizableHandle disableDoubleClick onDoubleClick={split.reset} />}
    </>
  );
}

function PanelGroupTabBar({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  const { state } = usePanelGroupContext();
  const isBottom = state.displayMode === "tabbar-bottom";
  return (
    <TabsList
      className={cn(
        "h-auto shrink-0 rounded-none border-border bg-card px-1 justify-start gap-0",
        isBottom ? "border-t" : "border-b",
        className,
      )}
    >
      {children}
    </TabsList>
  );
}

function PanelGroupTabBarTab({
  panelId,
  title,
  className,
}: {
  panelId: string;
  title: string;
  className?: string;
}) {
  return (
    <TabsTrigger
      value={panelId}
      className={cn(
        "h-7 rounded-none px-3 text-[0.917rem] data-[state=active]:bg-background data-[state=active]:shadow-none border-b-2 border-transparent data-[state=active]:border-primary",
        className,
      )}
    >
      {title}
    </TabsTrigger>
  );
}

function PanelGroupTabPanels({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <div className={cn("flex-1 min-h-0 overflow-hidden", className)}>{children}</div>;
}

function PanelGroupTabContent({
  panelId,
  children,
}: {
  panelId: string;
  children: React.ReactNode;
}) {
  return (
    <TabsContent value={panelId} className="mt-0 h-full data-[state=inactive]:hidden">
      {children}
    </TabsContent>
  );
}

function PanelGroupSidebar({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return <div className={cn("flex flex-col w-full h-full", className)}>{children}</div>;
}

function PanelGroupSidebarItem({
  panelId,
  title,
  children,
  className,
}: {
  panelId: string;
  title: string;
  children: React.ReactNode;
  className?: string;
}) {
  const { state, actions } = usePanelGroupContext();
  const isOpen = state.activeTabId === panelId;
  return (
    <Collapsible
      open={isOpen}
      onOpenChange={(open) => {
        if (open) actions.setActiveTab(panelId);
      }}
      className={cn("flex flex-col", className)}
    >
      <CollapsibleTrigger className="flex items-center gap-1 px-2 h-7 w-full text-left text-[0.917rem] uppercase tracking-wide text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard shrink-0">
        <ChevronRight
          size={12}
          className={cn(
            "transition-transform duration-[var(--motion-fast)] ease-standard shrink-0",
            isOpen && "rotate-90",
          )}
        />
        {title}
      </CollapsibleTrigger>
      <CollapsibleContent className="flex-1 min-h-0 overflow-hidden">{children}</CollapsibleContent>
    </Collapsible>
  );
}

const PanelGroupTabBarExport = Object.assign(PanelGroupTabBar, {
  Tab: PanelGroupTabBarTab,
});

const PanelGroupSidebarExport = Object.assign(PanelGroupSidebar, {
  Item: PanelGroupSidebarItem,
});

export const PanelGroup = {
  Provider: PanelGroupProvider,
  Split: PanelGroupSplit,
  SplitItem: PanelGroupSplitItem,
  TabBar: PanelGroupTabBarExport,
  TabPanels: PanelGroupTabPanels,
  TabContent: PanelGroupTabContent,
  Sidebar: PanelGroupSidebarExport,
};

export { Tabs as PanelGroupTabsRoot };
