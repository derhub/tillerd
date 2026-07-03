// Placeholder right dock region. Structural for now -- real content is a later
// change; this ships the toggleable, resizable region and its layout slot.
export function RightDock() {
  return (
    <aside data-testid="right-dock" className="h-full w-full overflow-hidden">
      <div className="flex h-full flex-col p-3">
        <p className="text-[0.833rem] text-muted-foreground/50 uppercase tracking-wider">
          Right dock
        </p>
      </div>
    </aside>
  );
}
