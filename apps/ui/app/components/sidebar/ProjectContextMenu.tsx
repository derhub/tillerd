import { ExternalLink, Pencil, Trash2 } from "lucide-react";

import { ContextMenuShell, MenuItem } from "~/components/sidebar/context-menu-shell";

export function ProjectContextMenu({
  at,
  allowMutations,
  onClose,
  onRename,
  onOpenInNewWindow,
  onDelete,
}: {
  at: { x: number; y: number };
  allowMutations: boolean;
  onClose: () => void;
  onRename: () => void;
  onOpenInNewWindow: () => void;
  onDelete: () => void;
}) {
  return (
    <ContextMenuShell at={at} onClose={onClose}>
      {allowMutations && (
        <MenuItem onClick={onRename}>
          <Pencil size={12} />
          <span>Rename</span>
        </MenuItem>
      )}
      <MenuItem onClick={onOpenInNewWindow}>
        <ExternalLink size={12} />
        <span>Open in new window</span>
      </MenuItem>
      {allowMutations && (
        <MenuItem onClick={onDelete}>
          <Trash2 size={12} />
          <span>Delete</span>
        </MenuItem>
      )}
    </ContextMenuShell>
  );
}
