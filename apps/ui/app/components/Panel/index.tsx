import { createContext, use, type ReactNode } from "react";
import { X } from "lucide-react";
import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { cn } from "~/lib/utils";

// ── Context ────────────────────────────────────────────────────────────────

type PanelActions = {
  split: (direction: "horizontal" | "vertical") => void;
  close: () => void;
};

type PanelContextValue = {
  state: { id: string; title: string };
  actions: PanelActions;
  meta: Record<string, never>;
};

const PanelContext = createContext<PanelContextValue | null>(null);

function usePanelContext() {
  const ctx = use(PanelContext);
  if (!ctx) throw new Error("Panel sub-components must be used inside Panel.Provider");
  return ctx;
}

// ── Sub-components ────────────────────────────────────────────────────────

function PanelProvider({
  children,
  id,
  title,
  actions,
}: {
  children: ReactNode;
  id: string;
  title: string;
  actions: PanelActions;
}) {
  return (
    <PanelContext value={{ state: { id, title }, actions, meta: {} }}>{children}</PanelContext>
  );
}

function PanelFrame({ children, className }: { children: ReactNode; className?: string }) {
  const { state } = usePanelContext();
  return (
    <div
      className={cn("group/panel flex flex-col h-full min-h-0 min-w-0", className)}
      data-panel-id={state.id}
    >
      {children}
    </div>
  );
}

function PanelHeader({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={cn("flex items-center shrink-0 px-4 gap-1.5", className)}
      style={{ height: "var(--panel-header-height, 2.5rem)" }}
    >
      {children}
    </div>
  );
}

function PanelTitle({ className }: { className?: string }) {
  const { state } = usePanelContext();
  return (
    <span
      className={cn(
        "truncate text-muted-foreground/60 flex-1 select-none text-[0.833rem] font-medium tracking-wider uppercase",
        className,
      )}
    >
      {state.title}
    </span>
  );
}

// Toolbar is hidden by default; appears on panel hover
function PanelToolbar({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={cn(
        "flex items-center gap-0.5 ml-auto shrink-0",
        "opacity-0 group-hover/panel:opacity-100 transition-opacity duration-100",
        className,
      )}
    >
      {children}
    </div>
  );
}

function PanelToolbarButton({
  icon,
  label,
  onClick,
  className,
}: {
  icon: ReactNode;
  label: string;
  onClick: () => void;
  className?: string;
}) {
  return (
    <Tooltip>
      <TooltipTrigger
        onClick={onClick}
        aria-label={label}
        className={cn(
          "flex items-center justify-center w-5 h-5 rounded-sm text-muted-foreground",
          "hover:text-foreground hover:bg-muted transition-colors",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          className,
        )}
      >
        {icon}
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}

function PanelCloseButton({ totalPanels }: { totalPanels: number }) {
  const { actions } = usePanelContext();
  if (totalPanels <= 1) return null;
  return <PanelToolbarButton icon={<X size={12} />} label="Close panel" onClick={actions.close} />;
}

function PanelContent({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={cn("flex-1 min-h-0 overflow-hidden", className)}>{children}</div>;
}

// ── Compound export ────────────────────────────────────────────────────────

const PanelToolbarExport = Object.assign(PanelToolbar, { Button: PanelToolbarButton });

export const Panel = {
  Provider: PanelProvider,
  Frame: PanelFrame,
  Header: PanelHeader,
  Title: PanelTitle,
  Toolbar: PanelToolbarExport,
  CloseButton: PanelCloseButton,
  Content: PanelContent,
};
