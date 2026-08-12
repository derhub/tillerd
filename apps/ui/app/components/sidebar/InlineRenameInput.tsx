import React from "react";
import { flushSync } from "react-dom";

import { cn } from "~/lib/utils";

interface InlineRenameInputProps {
  initialValue: string;
  onConfirm: (newValue: string) => void;
  onCancel: () => void;
  isProject?: boolean;
  restoreFocus?: () => HTMLElement | null;
}

export function InlineRenameInput({
  initialValue,
  onConfirm,
  onCancel,
  isProject = false,
  restoreFocus,
}: InlineRenameInputProps) {
  const [value, setValue] = React.useState(initialValue);
  const [error, setError] = React.useState(false);
  const inputRef = React.useRef<HTMLInputElement>(null);
  // Captured pre-focus: Escape must hand focus back to the row that launched the rename.
  const triggerRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => {
    triggerRef.current = document.activeElement as HTMLElement | null;
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") {
      if (isProject && !value.trim()) {
        setError(true);
        return;
      }
      onConfirm(value);
    } else if (e.key === "Escape") {
      const trigger = triggerRef.current;
      flushSync(onCancel);
      const replacement = restoreFocus?.();
      if (replacement?.isConnected) replacement.focus();
      else if (trigger?.isConnected) trigger.focus();
    }
  };

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setValue(e.target.value);
    if (error && e.target.value.trim()) {
      setError(false);
    }
  };

  return (
    <input
      ref={inputRef}
      type="text"
      value={value}
      onChange={handleChange}
      onKeyDown={handleKeyDown}
      onBlur={onCancel}
      className={cn(
        "flex-1 px-2 py-1 text-[0.833rem] rounded-sm border-none bg-muted focus:outline-none focus:ring-1 focus:ring-ring",
        error && "ring-1 ring-red-500",
      )}
      aria-label="Rename input"
      data-testid="inline-rename-input"
    />
  );
}
