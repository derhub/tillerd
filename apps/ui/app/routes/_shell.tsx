import { AppShell } from "~/components/AppShell";
import { API_BASE } from "~/lib/serverUrl";
import { DesktopHostProvider } from "~/lib/useDesktopHost";
import type { Route } from "./+types/_shell";

type Session = { id: string; cwd?: string };

export async function clientLoader(): Promise<{ sessions: Session[] }> {
  try {
    const res = await fetch(`${API_BASE}/api/sessions`);
    if (!res.ok) return { sessions: [] };
    return res.json() as Promise<{ sessions: Session[] }>;
  } catch {
    return { sessions: [] };
  }
}

export default function Shell({ loaderData }: Route.ComponentProps) {
  return (
    <DesktopHostProvider>
      <AppShell sessions={loaderData?.sessions ?? []} />
    </DesktopHostProvider>
  );
}
