import { type ErrorComponentProps, useRouter } from "@tanstack/react-router";

export function DefaultErrorBoundary({ error }: ErrorComponentProps) {
  const router = useRouter();
  return (
    <div
      className="h-full w-full flex flex-col items-center justify-center gap-3 p-6 text-center"
      data-testid="route-error"
    >
      <p className="text-sm font-medium text-foreground">Something went wrong</p>
      <p className="max-w-md text-[0.833rem] text-muted-foreground break-words">{error.message}</p>
      <button
        type="button"
        onClick={() => void router.invalidate()}
        className="px-2 h-6 text-[0.833rem] rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard"
      >
        Retry
      </button>
    </div>
  );
}
