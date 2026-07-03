import { Outlet, useNavigate, useParams, useSearch } from "@tanstack/react-router";
import { Undo2 } from "lucide-react";
import React from "react";

import { CommandCenter } from "~/components/command/CommandCenter";
import { ServiceHealthIndicator } from "~/components/health/ServiceHealthIndicator";
import { NotificationIndicator } from "~/components/notifications/NotificationIndicator";
import { SettingsPanel, SETTINGS_OPEN_EVENT } from "~/components/settings/SettingsPanel";
import { BottomDock } from "~/components/shell/BottomDock";
import { useArmReattachOnClose } from "~/components/shell/hooks/useArmReattachOnClose";
import { useDetachedPanels } from "~/components/shell/hooks/useDetachedPanels";
import { useMenuNavigation } from "~/components/shell/hooks/useMenuNavigation";
import { useTitleBarCommands } from "~/components/shell/hooks/useTitleBarCommands";
import { RightDock } from "~/components/shell/RightDock";
import { DetachedPanelsContext } from "~/components/shell/shellContext";
import { TitleBar } from "~/components/shell/TitleBar";
import { SessionSidebar } from "~/components/sidebar/SessionSidebar";
import { WorkspaceSwitcher } from "~/components/sidebar/WorkspaceSwitcher";
import { DetachedWindow } from "~/components/terminal/DetachedWindow";
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "~/components/ui/resizable";
import { Skeleton } from "~/components/ui/skeleton";
import { ACTION } from "~/lib/commands/ids";
import { CommandRegistryProvider, RegisterHandlers } from "~/lib/commands/registry";
import { NotificationsProvider } from "~/lib/notifications/context";
import { SessionContext } from "~/lib/sessionContext";
import { SettingsProvider } from "~/lib/settings/context";
import { usePanelVisible } from "~/lib/store";
import { DesktopHostProvider } from "~/lib/useDesktopHost";
import { parseWindowIntent, type WindowIntent } from "~/lib/windows";
import { closeSelf, emitReattachProject, emitReattachWorkspace } from "~/lib/windows";

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

// Registers the title-bar toggle command handlers and seeds their context keys.
// Rendered inside the command registry provider so `useCommand` can register.
function TitleBarCommands(): null {
  useTitleBarCommands();
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

  const [leftVisible] = usePanelVisible("left");
  const [rightVisible] = usePanelVisible("right");
  const [bottomVisible] = usePanelVisible("bottom");

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

  return (
    <DesktopHostProvider>
      <SettingsProvider>
        <NotificationsProvider>
          <CommandRegistryProvider>
            <SessionContext value={{ sessionId, status, setStatus }}>
              <DetachedPanelsContext value={{ detached, detach, reattach }}>
                <RegisterHandlers handlers={navHandlers} />
                <TitleBarCommands />
                <CommandCenter />
                <div className="h-dvh w-full flex flex-col overflow-hidden">
                  <TitleBar />
                  <div className="flex-1 min-h-0">
                    <ResizablePanelGroup orientation="vertical">
                      <ResizablePanel minSize="20%" className="min-h-0">
                        <ResizablePanelGroup orientation="horizontal">
                          {leftVisible && (
                            <ResizablePanel defaultSize="224px" minSize="180px" maxSize="360px">
                              <aside className="h-full w-full overflow-hidden border-r border-border/40">
                                <React.Suspense
                                  fallback={
                                    <div
                                      className="h-full w-full p-3"
                                      data-testid="sidebar-skeleton"
                                    >
                                      <Skeleton className="h-full w-full" />
                                    </div>
                                  }
                                >
                                  {isProjectWindow ? (
                                    <SessionSidebar activeProjectId={projectWindowId} />
                                  ) : (
                                    <WorkspaceSwitcher initialWorkspaceId={workspaceWindowId} />
                                  )}
                                </React.Suspense>
                              </aside>
                            </ResizablePanel>
                          )}
                          {leftVisible && <ResizableHandle />}
                          <ResizablePanel minSize="30%" className="min-w-0">
                            <div className="h-full w-full min-w-0 pt-px relative">
                              <Outlet />
                              <div className="absolute bottom-2 right-2 z-50 flex items-center gap-2">
                                {(isProjectWindow || isWorkspaceWindow) && (
                                  <button
                                    type="button"
                                    onClick={() => void closeSelf()}
                                    aria-label="Re-attach"
                                    className="flex items-center gap-1 px-2 h-6 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard"
                                  >
                                    <Undo2 size={12} />
                                    <span>Re-attach</span>
                                  </button>
                                )}
                                <NotificationIndicator />
                                <SettingsPanel />
                                <ServiceHealthIndicator />
                              </div>
                            </div>
                          </ResizablePanel>
                          {rightVisible && <ResizableHandle />}
                          {rightVisible && (
                            <ResizablePanel defaultSize="256px" minSize="180px" maxSize="480px">
                              <RightDock />
                            </ResizablePanel>
                          )}
                        </ResizablePanelGroup>
                      </ResizablePanel>
                      {bottomVisible && <ResizableHandle />}
                      {bottomVisible && (
                        <ResizablePanel defaultSize="200px" minSize="120px" maxSize="60%">
                          <BottomDock />
                        </ResizablePanel>
                      )}
                    </ResizablePanelGroup>
                  </div>
                </div>
              </DetachedPanelsContext>
            </SessionContext>
          </CommandRegistryProvider>
        </NotificationsProvider>
      </SettingsProvider>
    </DesktopHostProvider>
  );
}
