import { X } from "lucide-react";
import React from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "~/components/ui/tooltip";
import { cn } from "~/lib/utils";

type PanelActions = {
  split: (direction: "horizontal" | "vertical") => void;
  close: () => void;
};

type PanelContextValue = {
  state: { id: string; title: string };
  actions: PanelActions;
  meta: Record<string, never>;
};

const PanelContext = React.createContext<PanelContextValue | null>(null);

function usePanelContext() {
  const ctx = React.use(PanelContext);
  if (!ctx) throw new Error("Panel sub-components must be used inside Panel.Provider");
  return ctx;
}

function PanelProvider({
  children,
  id,
  title,
  actions,
}: {
  children: React.ReactNode;
  id: string;
  title: string;
  actions: PanelActions;
}) {
  return (
    <PanelContext value={{ state: { id, title }, actions, meta: {} }}>{children}</PanelContext>
  );
}

function PanelFrame({
  children,
  className,
  isClosing,
  isDropTarget,
  isFocused,
  onDragOver,
  onDragLeave,
  onDrop,
}: {
  children: React.ReactNode;
  className?: string;
  isClosing?: boolean;
  isDropTarget?: boolean;
  isFocused?: boolean;
  onDragOver?: (e: React.DragEvent) => void;
  onDragLeave?: (e: React.DragEvent) => void;
  onDrop?: (e: React.DragEvent) => void;
}) {
  const { state } = usePanelContext();
  // Opacity-only lifecycle fade (ui-panel-compound "Panel lifecycle motion"): a leaf fades in on
  // its first paint via this mount flag, and a closing leaf fades out before the caller actually
  // removes it from the tree (see PanelContent's closingLeafIds delay). No size/layout animation.
  const [mounted, setMounted] = React.useState(false);
  React.useEffect(() => {
    const raf = requestAnimationFrame(() => setMounted(true));
    return () => cancelAnimationFrame(raf);
  }, []);
  return (
    <div
      className={cn(
        "group/panel flex flex-col h-full min-h-0 min-w-0 focus:outline-none",
        "transition-opacity duration-[var(--motion-fast)] ease-standard motion-reduce:transition-none",
        isClosing ? "opacity-0 pointer-events-none" : mounted ? "opacity-100" : "opacity-0",
        // Drop-target ring is the loud full-primary edge; the focused-pane ring is a quieter inset
        // so the two states read differently when a drag lands on the focused pane.
        isDropTarget
          ? "ring-1 ring-inset ring-primary"
          : isFocused && "ring-1 ring-inset ring-ring/50",
        className,
      )}
      tabIndex={-1}
      data-panel-id={state.id}
      data-state={isClosing ? "closing" : mounted ? "entered" : "entering"}
      data-testid={isDropTarget ? "panel-drop-target-active" : undefined}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      {children}
    </div>
  );
}

function PanelHeader({
  children,
  className,
  draggable,
  onDragStart,
  onDragEnd,
}: {
  children: React.ReactNode;
  className?: string;
  draggable?: boolean;
  onDragStart?: (e: React.DragEvent) => void;
  onDragEnd?: (e: React.DragEvent) => void;
}) {
  return (
    <div
      className={cn(
        "flex items-center shrink-0 px-4 gap-1.5",
        draggable && "cursor-grab active:cursor-grabbing",
        className,
      )}
      style={{ height: "var(--panel-header-height, 2.5rem)" }}
      draggable={draggable}
      onDragStart={onDragStart}
      onDragEnd={onDragEnd}
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

function PanelToolbar({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div
      className={cn(
        "flex items-center gap-0.5 ml-auto shrink-0",
        "opacity-0 group-hover/panel:opacity-100 transition-opacity duration-[var(--motion-fast)] ease-standard",
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
  icon: React.ReactNode;
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
          "hover:text-foreground hover:bg-muted transition-colors duration-[var(--motion-fast)] ease-standard",
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

// Close is content-dependent (surface-lifecycle spec): a terminal leaf can always be closed (it
// resets to the empty picker in place, even as the only pane); an empty leaf can be closed only
// when it is not the sole leaf (closing removes it). The caller passes the resolved `canClose` so
// the button hides exactly for the sole-empty case.
function PanelCloseButton({ canClose }: { canClose: boolean }) {
  const { actions } = usePanelContext();
  if (!canClose) return null;
  return (
    <PanelToolbarButton
      icon={<X className="size-[var(--icon-md)]" />}
      label="Close panel"
      onClick={actions.close}
    />
  );
}

function PanelContent({ children, className }: { children: React.ReactNode; className?: string }) {
  return <div className={cn("flex-1 min-h-0 overflow-hidden", className)}>{children}</div>;
}

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
