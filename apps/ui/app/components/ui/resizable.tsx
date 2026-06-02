import * as ResizablePrimitive from "react-resizable-panels";

import { cn } from "~/lib/utils";

function ResizablePanelGroup({ className, ...props }: ResizablePrimitive.GroupProps) {
  return (
    <ResizablePrimitive.Group
      data-slot="resizable-panel-group"
      className={cn("flex h-full w-full aria-[orientation=vertical]:flex-col", className)}
      {...props}
    />
  );
}

function ResizablePanel({ ...props }: ResizablePrimitive.PanelProps) {
  return <ResizablePrimitive.Panel data-slot="resizable-panel" {...props} />;
}

function ResizableHandle({
  withHandle,
  className,
  ...props
}: ResizablePrimitive.SeparatorProps & {
  withHandle?: boolean;
}) {
  return (
    <ResizablePrimitive.Separator
      data-slot="resizable-handle"
      className={cn(
        // Base: 1px line, theme border color, 12px hit area
        "group relative flex w-px items-center justify-center bg-border",
        "after:absolute after:inset-y-0 after:left-1/2 after:w-3 after:-translate-x-1/2",
        // Cursor: col-resize by default, row-resize for horizontal
        "cursor-col-resize",
        "aria-[orientation=horizontal]:cursor-row-resize",
        // Hover: line brightens slightly
        "transition-[opacity,width] duration-100",
        "hover:opacity-100 opacity-60",
        // Active (dragging): full opacity — width handled in app.css
        "data-[separator=active]:opacity-100",
        // Focus
        "focus-visible:outline-hidden focus-visible:opacity-100",
        // Horizontal orientation overrides
        "aria-[orientation=horizontal]:h-px aria-[orientation=horizontal]:w-full",
        "aria-[orientation=horizontal]:after:left-0 aria-[orientation=horizontal]:after:h-3",
        "aria-[orientation=horizontal]:after:w-full aria-[orientation=horizontal]:after:translate-x-0",
        "aria-[orientation=horizontal]:after:-translate-y-1/2",
        className,
      )}
      {...props}
    >
      {withHandle && (
        // Grip pill: hidden at rest, appears on hover and while dragging
        <div
          className={cn(
            "z-10 h-8 w-1 shrink-0 rounded-full bg-border",
            "opacity-0 group-hover:opacity-100 transition-opacity duration-100",
            "group-data-[resize-handle-active]:opacity-100",
          )}
        />
      )}
    </ResizablePrimitive.Separator>
  );
}

export { ResizableHandle, ResizablePanel, ResizablePanelGroup };
