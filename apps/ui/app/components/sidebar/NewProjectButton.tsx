import { FolderPlus } from "lucide-react";

import { cn } from "~/lib/utils";

export function NewProjectButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex items-center gap-1.5 px-2 h-6 text-[0.75rem] rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
        "text-muted-foreground hover:text-foreground hover:bg-muted",
      )}
      title="New project"
    >
      <FolderPlus strokeWidth={2} className="size-[var(--icon-sm)]" />
      <span>New project</span>
    </button>
  );
}
