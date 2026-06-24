export function reorderByDrop(ids: string[], sourceId: string, targetId: string): string[] {
  if (sourceId === targetId) return ids;
  const next = [...ids];
  const from = next.indexOf(sourceId);
  const to = next.indexOf(targetId);
  if (from < 0 || to < 0) return ids;
  next.splice(to, 0, next.splice(from, 1)[0]);
  return next;
}
