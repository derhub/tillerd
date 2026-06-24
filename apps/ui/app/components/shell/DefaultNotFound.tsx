import { useRouter } from "@tanstack/react-router";

// Stale deep links produce unmatched routes; recoverable fallback over bare router default.
export function DefaultNotFound() {
  const router = useRouter();
  return (
    <div
      className="h-full w-full flex flex-col items-center justify-center gap-3 p-6 text-center"
      data-testid="route-not-found"
    >
      <p className="text-sm font-medium text-foreground">Page not found</p>
      <button
        type="button"
        onClick={() => void router.navigate({ to: "/" })}
        className="px-2 h-6 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard"
      >
        Go home
      </button>
    </div>
  );
}
