// Placeholder bottom dock region. Structural for now -- real content is a later
// change; this ships the toggleable region and its layout slot.
export function BottomDock() {
  return (
    <div
      data-testid="bottom-dock"
      className="h-48 shrink-0 overflow-hidden border-t border-border/40"
    >
      <div className="flex h-full flex-col p-3">
        <p className="text-[0.833rem] text-muted-foreground/50 uppercase tracking-wider">
          Bottom dock
        </p>
      </div>
    </div>
  );
}
