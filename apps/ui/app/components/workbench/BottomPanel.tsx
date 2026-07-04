import { LogViewer } from "~/components/logs/LogViewer";
import { NotificationPanel } from "~/components/notifications/NotificationIndicator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/components/ui/tabs";
import { useNotifications } from "~/lib/notifications/context";
import { useBottomPanelTab } from "~/lib/workbench";

// Bottom panel: a tab strip over the existing Logs and Notifications surfaces. Logs
// stays mounted (`keepMounted`) so its live tail and scroll position survive a tab
// switch. The active tab persists via the workbench settings store.
export function BottomPanel() {
  const [tab, setTab] = useBottomPanelTab();
  const { items } = useNotifications();

  return (
    <Tabs
      value={tab}
      onValueChange={(value) => setTab(String(value))}
      className="h-full min-h-0 w-full gap-0 overflow-hidden bg-background"
    >
      <TabsList
        variant="line"
        className="h-8 w-full shrink-0 justify-start gap-1 rounded-none border-b border-border/40 px-2"
      >
        <TabsTrigger value="logs" className="flex-none px-2 text-[0.833rem]">
          Logs
        </TabsTrigger>
        <TabsTrigger value="notifications" className="flex-none px-2 text-[0.833rem]">
          Notifications
        </TabsTrigger>
      </TabsList>
      <TabsContent value="logs" keepMounted className="min-h-0 flex-1 overflow-hidden">
        <LogViewer />
      </TabsContent>
      <TabsContent value="notifications" className="min-h-0 flex-1 overflow-auto">
        <div data-testid="notification-panel">
          <NotificationPanel items={items} />
        </div>
      </TabsContent>
    </Tabs>
  );
}
