import React from "react";

const TICK_MS = 30_000;

// One coarse re-render tick per panel area (ui-panel-compound "Panel title content"): forces
// panel titles to recompute their "Xm"/"Xh Ym" elapsed text every 30s. This never refetches
// surface data -- spawnedAt is already fetched; only the derived display string goes stale.
export function useElapsedTick(): number {
  const [now, setNow] = React.useState(() => Date.now());
  React.useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(id);
  }, []);
  return now;
}
