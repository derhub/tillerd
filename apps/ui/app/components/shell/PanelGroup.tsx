import { ChevronRight } from "lucide-react";
import React from "react";

import type { DisplayMode } from "~/lib/panelTree";

import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "~/components/ui/collapsible";
import {
  ResizablePanelGroup as ResizablePanelGroupBase,
  ResizablePanel,
  ResizableHandle,
} from "~/components/ui/resizable";
import { TabsList, TabsTrigger, Tabs, TabsContent } from "~/components/ui/tabs";
import { cn } from "~/lib/utils";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ResizablePanelGroup = ResizablePanelGroupBase as React.ComponentType<any>;

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
  autoSaveId,
  className,
}: {
  children: React.ReactNode;
  autoSaveId?: string;
  className?: string;
}) {
  const { state } = usePanelGroupContext();
  return (
    <ResizablePanelGroup
      orientation={state.direction}
      {...(autoSaveId ? { autoSaveId } : {})}
      className={cn("h-full", className)}
    >
      {children}
    </ResizablePanelGroup>
  );
}

function PanelGroupSplitItem({
  children,
  minSize,
  isLast,
}: {
  children: React.ReactNode;
  minSize?: number;
  isLast: boolean;
}) {
  // defaultSize is uniform across every split item on purpose (ui-panel-compound "Divider
  // reset"): react-resizable-panels' Separator resets its adjacent Panels to their defaultSize
  // on double-click, so an equal defaultSize ratio here is what makes that reset land on an
  // equal split. splitNode always produces exactly two children per group, so "its adjacent
  // Panels" is the whole group. Do not diverge this value per item without re-deriving 100/n.
  return (
    <>
      <ResizablePanel minSize={minSize ?? 10} defaultSize={33}>
        {children}
      </ResizablePanel>
      {!isLast && <ResizableHandle />}
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
