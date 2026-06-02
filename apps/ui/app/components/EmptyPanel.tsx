import { TerminalIcon, GitCompareIcon, LayoutListIcon } from "lucide-react";
import { cn } from "~/lib/utils";
import type { PanelContent } from "~/lib/panelTree";

type ContentOption = {
  type: PanelContent["type"];
  label: string;
  icon: React.ReactNode;
};

const OPTIONS: ContentOption[] = [
  { type: "terminal", label: "Terminal", icon: <TerminalIcon size={12} /> },
  { type: "diff", label: "Changes", icon: <GitCompareIcon size={12} /> },
  { type: "sidebar", label: "Sessions", icon: <LayoutListIcon size={12} /> },
];

export function EmptyPanel({ onSelect }: { onSelect: (content: PanelContent) => void }) {
  return (
    <div className="flex flex-col h-full pt-[20%] px-4">
      <p className="text-[0.833rem] text-muted-foreground/50 mb-2 uppercase tracking-wider">
        Select type
      </p>
      <div className="flex flex-col gap-px">
        {OPTIONS.map((opt) => (
          <button
            key={opt.type}
            type="button"
            onClick={() =>
              onSelect(
                opt.type === "terminal"
                  ? { type: "terminal", sessionId: null }
                  : opt.type === "diff"
                    ? { type: "diff", sessionId: null }
                    : { type: "sidebar" },
              )
            }
            className={cn(
              "flex items-center gap-2 px-2 h-7 rounded-sm text-[0.917rem] text-left",
              "text-muted-foreground hover:text-foreground hover:bg-muted transition-colors",
            )}
          >
            {opt.icon}
            {opt.label}
          </button>
        ))}
      </div>
    </div>
  );
}
