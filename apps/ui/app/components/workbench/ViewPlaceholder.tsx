import type { LucideIcon } from "lucide-react";

// Production empty-state for sidebar views whose content lands in a later task
// (search, commands, templates). Header mirrors the sessions view's chrome; body
// is a muted, centered empty state -- no debug or lorem text.
export function ViewPlaceholder({
  icon: Icon,
  title,
  hint,
}: {
  icon: LucideIcon;
  title: string;
  hint: string;
}) {
  return (
    <div className="flex h-full flex-col">
      <div className="flex h-8 shrink-0 items-center px-3">
        <span className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground">
          {title}
        </span>
      </div>
      <div className="flex flex-1 flex-col items-center justify-center gap-2 px-4 text-center">
        <Icon className="size-[var(--icon-lg)] text-muted-foreground/40" aria-hidden />
        <p className="text-[0.833rem] text-muted-foreground">{hint}</p>
      </div>
    </div>
  );
}
