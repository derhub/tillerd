import { AppShell } from "~/components/AppShell";
import { DesktopHostProvider } from "~/lib/useDesktopHost";

export default function Shell() {
  return (
    <DesktopHostProvider>
      <AppShell />
    </DesktopHostProvider>
  );
}
