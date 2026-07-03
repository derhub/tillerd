import { ExternalLink, Pencil, Trash2 } from "lucide-react";

import { ContextMenuShell, MenuItem } from "~/components/sidebar/context-menu-shell";

export function ProjectContextMenu({
  at,
  allowRename,
  allowDelete,
  onClose,
  onRename,
  onOpenInNewWindow,
  onDelete,
}: {
  at: { x: number; y: number };
  // Rename gating is UX-only (no server guard); delete gating mirrors the domain
  // guard table (stateModel `can`), so the two are separate inputs.
  allowRename: boolean;
  allowDelete: boolean;
  onClose: () => void;
  onRename: () => void;
  onOpenInNewWindow: () => void;
  onDelete: () => void;
}) {
  return (
    <ContextMenuShell at={at} onClose={onClose}>
      {allowRename && (
        <MenuItem onClick={onRename}>
          <Pencil size={12} />
          <span>Rename</span>
        </MenuItem>
      )}
      <MenuItem onClick={onOpenInNewWindow}>
        <ExternalLink size={12} />
        <span>Open in new window</span>
      </MenuItem>
      {allowDelete && (
        <MenuItem onClick={onDelete}>
          <Trash2 size={12} />
          <span>Delete</span>
        </MenuItem>
      )}
    </ContextMenuShell>
  );
}
