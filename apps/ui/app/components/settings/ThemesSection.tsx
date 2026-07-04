import type { ThemeView } from "@tillerd/client-bindings";

import { useMutation, useQuery } from "@tanstack/react-query";
import { command, query } from "@tillerd/client-bindings";
import { Download, Trash2, Upload } from "lucide-react";
import React from "react";

import { Button } from "~/components/ui/button";
import { cn } from "~/lib/utils";

export interface ThemesListProps {
  themes: ThemeView[];
  activeId: string | null;
  onActivate: (id: string) => void;
  onExport: (id: string) => void;
  onDelete: (id: string) => void;
}

export function ThemesList({ themes, activeId, onActivate, onExport, onDelete }: ThemesListProps) {
  if (themes.length === 0) {
    return <p className="text-muted-foreground/60 italic text-[0.917rem]">No themes</p>;
  }

  return (
    <ul className="flex flex-col gap-0.5" data-testid="themes-list">
      {themes.map((t) => (
        <li
          key={t.id}
          data-testid="theme-row"
          data-theme-id={t.id}
          data-theme-origin={t.origin}
          className="flex items-center gap-2 h-7 px-1"
        >
          <button
            type="button"
            onClick={() => onActivate(t.id)}
            className={cn(
              "flex-1 text-left truncate text-[0.917rem] px-2 py-0.5 rounded-sm transition-colors duration-[var(--motion-fast)] ease-standard",
              t.id === activeId
                ? "font-medium bg-muted text-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-muted",
            )}
          >
            {t.name}
          </button>
          <span className="text-[0.75rem] text-muted-foreground/60 shrink-0">{t.origin}</span>
          {t.id === activeId && (
            <span
              data-testid="theme-active-badge"
              className="text-[0.75rem] uppercase tracking-[0.05em] text-primary shrink-0"
            >
              Active
            </span>
          )}
          <button
            type="button"
            aria-label={`Export ${t.name}`}
            onClick={() => onExport(t.id)}
            className="flex items-center justify-center w-6 h-6 rounded-sm text-muted-foreground hover:text-foreground hover:bg-muted"
          >
            <Download size={12} />
          </button>
          {/* Prebuilt themes carry no delete affordance -- the spec's prebuilt guard. */}
          {t.origin === "custom" && (
            <button
              type="button"
              aria-label={`Delete ${t.name}`}
              onClick={() => onDelete(t.id)}
              className="flex items-center justify-center w-6 h-6 rounded-sm text-muted-foreground hover:text-destructive hover:bg-destructive/10"
            >
              <Trash2 size={12} />
            </button>
          )}
        </li>
      ))}
    </ul>
  );
}

function downloadBytes(bytes: number[], filename: string): void {
  const blob = new Blob([new Uint8Array(bytes)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// Plain async helper (not a hook/component) -- the no-async-in-component rule exempts these;
// components fire mutations via mutate(), never await mutateAsync() or .then() themselves.
function importThemeFile(
  file: File,
  importTheme: {
    mutateAsync: (args: {
      id: string;
      name: string;
      origin: string;
      dataJson: string | null;
    }) => Promise<unknown>;
  },
  onError: (message: string) => void,
): void {
  void file
    .text()
    .then((dataJson) =>
      importTheme.mutateAsync({
        id: crypto.randomUUID(),
        name: file.name.replace(/\.json$/i, ""),
        origin: "custom",
        dataJson,
      }),
    )
    .catch(() => onError(`Could not import ${file.name}`));
}

export function ThemesSection() {
  const { data: themes } = useQuery(query("themeList"));
  const { data: active } = useQuery(query("themeGetActive"));
  const activeId = active?.id ?? null;

  const [importError, setImportError] = React.useState<string | null>(null);
  const fileInputRef = React.useRef<HTMLInputElement>(null);

  const activateTheme = useMutation(command("themeActivate"));
  const discardTheme = useMutation(command("themeDiscard"));
  const exportTheme = useMutation(command("themeExport"));
  const importTheme = useMutation(command("themeImport"));

  const handleExport = React.useCallback(
    (id: string) => {
      const t = themes?.find((row) => row.id === id);
      exportTheme.mutate(
        { id },
        {
          onSuccess: (bytes) => {
            if (bytes) downloadBytes(bytes, `${t?.name ?? id}.theme.json`);
          },
        },
      );
    },
    [themes, exportTheme],
  );

  const handleImportFile = React.useCallback(
    (file: File) => {
      setImportError(null);
      importThemeFile(file, importTheme, setImportError);
    },
    [importTheme],
  );

  return (
    <section aria-labelledby="settings-themes-heading" className="flex flex-col gap-3 max-w-md">
      <div className="flex items-center justify-between gap-3">
        <h2
          id="settings-themes-heading"
          className="text-[0.75rem] font-medium uppercase tracking-[0.05em] text-muted-foreground"
        >
          Themes
        </h2>
        <div className="flex items-center gap-1">
          <input
            ref={fileInputRef}
            type="file"
            accept="application/json"
            className="hidden"
            onChange={(e) => {
              const file = e.target.files?.[0];
              if (file) handleImportFile(file);
              e.target.value = "";
            }}
          />
          <Button variant="ghost" size="xs" onClick={() => fileInputRef.current?.click()}>
            <Upload size={11} />
            Import
          </Button>
        </div>
      </div>

      {importError && <p className="text-destructive text-[0.833rem]">{importError}</p>}

      <ThemesList
        themes={themes ?? []}
        activeId={activeId}
        onActivate={(id) => activateTheme.mutate({ id })}
        onExport={handleExport}
        onDelete={(id) => discardTheme.mutate({ id })}
      />
    </section>
  );
}
