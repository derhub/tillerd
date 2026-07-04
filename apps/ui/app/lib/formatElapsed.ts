// Panel title elapsed-since-spawn text (ui-panel-compound "Panel title content"): under a
// minute reads as "now"; under an hour as whole minutes; beyond that as hours + minutes.
export function formatElapsed(spawnedAtMs: number, nowMs: number): string {
  const diffSec = Math.max(0, Math.floor((nowMs - spawnedAtMs) / 1000));
  if (diffSec < 60) return "now";
  const minutes = Math.floor(diffSec / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ${minutes % 60}m`;
}
