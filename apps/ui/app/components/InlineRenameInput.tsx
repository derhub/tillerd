import { useEffect, useRef, useState } from "react";
import { cn } from "~/lib/utils";

interface InlineRenameInputProps {
  initialValue: string;
  onConfirm: (newValue: string) => void;
  onCancel: () => void;
  isProject?: boolean;
}

export function InlineRenameInput({
  initialValue,
  onConfirm,
  onCancel,
  isProject = false,
}: InlineRenameInputProps) {
  const [value, setValue] = useState(initialValue);
  const [error, setError] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
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
      onCancel();
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
        "flex-1 px-2 py-1 text-sm rounded-sm border-none bg-muted focus:outline-none focus:ring-1 focus:ring-ring",
        error && "ring-1 ring-red-500",
      )}
      aria-label="Rename input"
      data-testid="inline-rename-input"
    />
  );
}
