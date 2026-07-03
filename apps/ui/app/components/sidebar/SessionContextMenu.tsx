import { Archive, Pencil, Trash2 } from "lucide-react";

import { ContextMenuShell, MenuItem } from "~/components/sidebar/context-menu-shell";

export function SessionContextMenu({
  at,
  onClose,
  onRename,
  onArchive,
  onDelete,
}: {
  at: { x: number; y: number };
  onClose: () => void;
  onRename: () => void;
  onArchive: () => void;
  onDelete: () => void;
}) {
  return (
    <ContextMenuShell at={at} onClose={onClose}>
      <MenuItem onClick={onRename}>
        <Pencil size={12} />
        <span>Rename</span>
      </MenuItem>
      <MenuItem onClick={onArchive}>
        <Archive size={12} />
        <span>Archive</span>
      </MenuItem>
      <MenuItem onClick={onDelete}>
        <Trash2 size={12} />
        <span>Delete</span>
      </MenuItem>
    </ContextMenuShell>
  );
}
