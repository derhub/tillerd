import { Outlet, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import React from "react";

import { CommandCenter } from "~/components/command/CommandCenter";
import { SETTINGS_OPEN_EVENT } from "~/components/settings/SettingsPanel";
import { useArmReattachOnClose } from "~/components/shell/hooks/useArmReattachOnClose";
import { useDetachedPanels } from "~/components/shell/hooks/useDetachedPanels";
import { useMenuCommands } from "~/components/shell/hooks/useMenuCommands";
import { useMenuNavigation } from "~/components/shell/hooks/useMenuNavigation";
import { useWorkbenchCommands } from "~/components/shell/hooks/useWorkbenchCommands";
import { DetachedPanelsContext } from "~/components/shell/shellContext";
import { TitleBar } from "~/components/shell/TitleBar";
import { DetachedWindow } from "~/components/terminal/DetachedWindow";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "~/components/ui/resizable";
import { TooltipProvider } from "~/components/ui/tooltip";
import { ActivityBar } from "~/components/workbench/ActivityBar";
import { BottomPanel } from "~/components/workbench/BottomPanel";
import { SidebarContainer } from "~/components/workbench/SidebarContainer";
import { StatusBar } from "~/components/workbench/StatusBar";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";
import { NotificationsProvider } from "~/lib/notifications/context";
import { SessionContext } from "~/lib/sessionContext";
import { SettingsProvider } from "~/lib/settings/context";
import { DesktopHostProvider } from "~/lib/useDesktopHost";
import { emitReattachProject, emitReattachWorkspace } from "~/lib/windows";
import { parseWindowIntent, type WindowIntent } from "~/lib/windows";
import {
  useBottomPanelSize,
  useBottomPanelVisible,
  useSidebarSize,
  useSidebarVisible,
} from "~/lib/workbench";

export function RootLayout() {
  const search = useSearch({ from: "__root__" });
  const intent = parseWindowIntent(`?${new URLSearchParams(search).toString()}`);

  if (intent.kind === "detached") {
    return (
      <DesktopHostProvider>
        <DetachedWindow sessionId={intent.sessionId} placement={intent.placement} />
      </DesktopHostProvider>
    );
  }

  return <ShellChrome intent={intent} />;
}

// Registers the workbench chrome command handlers and seeds their context keys.
// Rendered inside the command registry provider so `useCommand` can register.
function WorkbenchCommands(): null {
  useWorkbenchCommands();
  return null;
}

// Dispatches native menu accelerators through the command registry. Rendered inside the
// provider (not called directly in ShellChrome) so `useCommands` sees the registered handlers --
// ShellChrome itself sits above CommandRegistryProvider in the tree.
function MenuCommands(): null {
  useMenuCommands();
  return null;
}

function ShellChrome({ intent }: { intent: Exclude<WindowIntent, { kind: "detached" }> }) {
  const isProjectWindow = intent.kind === "project";
  const isWorkspaceWindow = intent.kind === "workspace";
  const projectWindowId = intent.kind === "project" ? intent.projectId : undefined;
  const workspaceWindowId = intent.kind === "workspace" ? intent.workspaceId : undefined;
  const intentSessionId = intent.kind === "project" ? intent.sessionId : null;

  const params = useParams({ strict: false }) as { id?: string };
  const sessionId = params.id ?? intentSessionId ?? null;
  const [status, setStatus] = React.useState("");

  const [sidebarVisible] = useSidebarVisible();
  const [sidebarSize, setSidebarSize] = useSidebarSize();
  const [bottomVisible] = useBottomPanelVisible();
  const [bottomSize, setBottomSize] = useBottomPanelSize();

  useMenuNavigation();

  const { detached, detach, reattach } = useDetachedPanels(sessionId, isProjectWindow);

  useArmReattachOnClose(projectWindowId, (id) => emitReattachProject({ projectId: id }));
  useArmReattachOnClose(workspaceWindowId, (id) => emitReattachWorkspace({ workspaceId: id }));

  const navigate = useNavigate();
  const navHandlers = React.useMemo(
    () => ({
      [ACTION.viewLogs]: () => void navigate({ to: "/logs" }),
      [ACTION.appSettings]: () => window.dispatchEvent(new CustomEvent(SETTINGS_OPEN_EVENT)),
    }),
    [navigate],
  );

  // Stable element so a resize-driven re-render of the shell never re-renders the
  // panel-area content (terminals). defaultSize is read once at mount by the panel.
  const contentPanel = React.useMemo(
    () => (
      <ResizablePanel minSize="30%" className="min-w-0">
        <div className="h-full w-full min-w-0 pt-px">
          <Outlet />
        </div>
      </ResizablePanel>
    ),
    [],
  );

  return (
    <DesktopHostProvider>
      <SettingsProvider>
        <NotificationsProvider>
          <CommandRegistryProvider>
            <SessionContext value={{ sessionId, status, setStatus }}>
              <DetachedPanelsContext value={{ detached, detach, reattach }}>
                <RegisterHandlers handlers={navHandlers} />
                <WorkbenchCommands />
                <MenuCommands />
                <CommandCenter />
                <TooltipProvider>
                  <div className="h-dvh w-full flex flex-col overflow-hidden">
                    <TitleBar />
                    <div className="flex-1 min-h-0 flex">
                      <ActivityBar />
                      <div className="flex-1 min-w-0">
                        <ResizablePanelGroup orientation="vertical">
                          <ResizablePanel minSize="20%" className="min-h-0">
                            <ResizablePanelGroup orientation="horizontal">
                              {sidebarVisible && (
                                <ResizablePanel
                                  defaultSize={`${sidebarSize}px`}
                                  minSize="180px"
                                  maxSize="360px"
                                  onResize={(size) => setSidebarSize(Math.round(size.inPixels))}
                                >
                                  <aside className="h-full w-full overflow-hidden">
                                    <SidebarContainer
                                      isProjectWindow={isProjectWindow}
                                      projectWindowId={projectWindowId}
                                      workspaceWindowId={workspaceWindowId}
                                    />
                                  </aside>
                                </ResizablePanel>
                              )}
                              {sidebarVisible && <ResizableHandle />}
                              {contentPanel}
                            </ResizablePanelGroup>
                          </ResizablePanel>
                          {bottomVisible && <ResizableHandle />}
                          {bottomVisible && (
                            <ResizablePanel
                              defaultSize={`${bottomSize}px`}
                              minSize="120px"
                              maxSize="60%"
                              onResize={(size) => setBottomSize(Math.round(size.inPixels))}
                            >
                              <BottomPanel />
                            </ResizablePanel>
                          )}
                        </ResizablePanelGroup>
                      </div>
                    </div>
                    <StatusBar showReattach={isProjectWindow || isWorkspaceWindow} />
                  </div>
                </TooltipProvider>
              </DetachedPanelsContext>
            </SessionContext>
          </CommandRegistryProvider>
        </NotificationsProvider>
      </SettingsProvider>
    </DesktopHostProvider>
  );
}
