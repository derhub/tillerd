import { Outlet } from "react-router";
import { AppShell } from "~/components/AppShell";
import type { Route } from "./+types/_shell";

type Session = { id: string; cwd?: string };

export async function clientLoader(): Promise<{ sessions: Session[] }> {
  try {
    const res = await fetch(
      `http://${window.location.hostname}:3000/api/sessions`,
    );
    if (!res.ok) return { sessions: [] };
    return res.json() as Promise<{ sessions: Session[] }>;
  } catch {
    return { sessions: [] };
  }
}

export default function Shell({ loaderData }: Route.ComponentProps) {
  return <AppShell sessions={loaderData.sessions} />;
}
