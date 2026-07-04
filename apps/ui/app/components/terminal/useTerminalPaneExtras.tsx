import type { FitAddon } from "@xterm/addon-fit";
import type { SearchAddon } from "@xterm/addon-search";
import type { IDisposable, Terminal } from "@xterm/xterm";

import React from "react";

import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
} from "~/components/ui/alert-dialog";
import { lazySearchAddon, lazyWebLinksAddon } from "~/lib/lazy";
import { emitBellNotification } from "~/lib/notifications/bell";
import { isMac } from "~/lib/platform";
import {
  useLiveTerminalTypography,
  useTerminalClipboardSettings,
  type TerminalTypography,
} from "~/lib/settings/useLiveTerminalTypography";
import {
  clearActiveTerminal,
  setActiveTerminal,
  type TerminalController,
} from "~/lib/terminal/activeTerminal";
import { classifyTerminalKey, linkModifierHeld } from "~/lib/terminal/keymap";
import { openExternalUrl } from "~/lib/terminal/open-external";
import { shouldConfirmPaste } from "~/lib/terminal/paste";
import { toSearchOptions } from "~/lib/terminal/search-decorations";
import { shellQuotePath } from "~/lib/terminal/shell-quote";
import { isDesktopHost } from "~/lib/transport/core";

import {
  TerminalSearchOverlay,
  type TerminalSearchController,
  type TerminalSearchResults,
} from "./TerminalSearchOverlay";

// Read the clipboard and either paste immediately or defer to a confirmation. Module-level (not a
// hook/component) so the async read stays outside React per the no-async-in-component rule.
function pasteFromClipboard(
  term: Terminal,
  confirmEnabled: boolean,
  onNeedsConfirm: (text: string) => void,
): void {
  void navigator.clipboard.readText().then((text) => {
    if (!text) return;
    if (shouldConfirmPaste(text, confirmEnabled)) onNeedsConfirm(text);
    else term.paste(text);
  });
}

interface AttachConfig {
  termRef: React.RefObject<Terminal | null>;
  fitRef: React.RefObject<FitAddon | null>;
  searchRef: React.RefObject<SearchAddon | null>;
  controllerRef: React.RefObject<TerminalController>;
  typoRef: React.RefObject<TerminalTypography>;
  copyOnSelectRef: React.RefObject<boolean>;
  containerRef: React.RefObject<HTMLDivElement | null>;
  writeInput: (text: string) => void;
  sessionId: string | null;
  sessionLabel: string | null;
  getSurfaceId: () => string | null;
  isDesktop: boolean;
  setResults: (r: TerminalSearchResults) => void;
  setHasSelection: (v: boolean) => void;
}

// Attach addons and handlers to a mounted terminal, returning a disposer. Module-level so its
// awaits/dynamic imports live outside React (mirrors native-banner's loadBannerDeps).
async function attachTerminalExtras(
  term: Terminal,
  fitAddon: FitAddon,
  cfg: AttachConfig,
): Promise<() => void> {
  cfg.termRef.current = term;
  cfg.fitRef.current = fitAddon;
  // Apply the current typography now: settings may have hydrated between construction and attach,
  // and the live effect only re-runs on a subsequent change.
  const t = cfg.typoRef.current;
  term.options.fontSize = t.fontSize;
  term.options.fontFamily = t.fontFamily;
  term.options.lineHeight = t.lineHeight;
  term.options.cursorStyle = t.cursorStyle;
  term.options.cursorBlink = t.cursorBlink;
  term.options.scrollback = t.scrollback;
  fitAddon.fit();
  setActiveTerminal(cfg.controllerRef.current);

  const [{ SearchAddon }, { WebLinksAddon }] = await Promise.all([
    lazySearchAddon(),
    lazyWebLinksAddon(),
  ]);
  const search = new SearchAddon();
  term.loadAddon(search);
  cfg.searchRef.current = search;
  const webLinks = new WebLinksAddon((e, uri) => {
    if (linkModifierHeld(e, isMac)) void openExternalUrl(uri);
  });
  term.loadAddon(webLinks);

  // OSC 8 hyperlinks the program emits directly (WebLinksAddon covers plain-text URLs).
  term.options.linkHandler = {
    activate: (e, text) => {
      if (linkModifierHeld(e, isMac)) void openExternalUrl(text);
    },
  };

  const disposables: IDisposable[] = [];
  disposables.push(
    search.onDidChangeResults((r) =>
      cfg.setResults({ resultIndex: r.resultIndex, resultCount: r.resultCount }),
    ),
  );
  disposables.push(
    term.onBell(
      () =>
        void emitBellNotification({
          sessionId: cfg.sessionId,
          surfaceId: cfg.getSurfaceId(),
          sessionLabel: cfg.sessionLabel,
        }),
    ),
  );
  disposables.push(
    term.onSelectionChange(() => {
      const sel = term.getSelection();
      cfg.setHasSelection(sel.length > 0);
      if (sel.length > 0 && cfg.copyOnSelectRef.current) void navigator.clipboard.writeText(sel);
    }),
  );

  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== "keydown") return true;
    const action = classifyTerminalKey(e, isMac);
    if (!action) return true;
    if (action === "copy") {
      const sel = term.getSelection();
      if (!sel) return true; // nothing selected: let the key through
      void navigator.clipboard.writeText(sel);
      return false;
    }
    if (action === "paste") {
      cfg.controllerRef.current.paste();
      return false;
    }
    cfg.controllerRef.current.openFind();
    return false;
  });

  let dropOff: (() => void) | undefined;
  if (cfg.isDesktop) {
    const { getCurrentWebview } = await import("@tauri-apps/api/webview");
    dropOff = await getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      const rect = cfg.containerRef.current?.getBoundingClientRect();
      // Drop position is physical pixels; the rect is CSS pixels.
      const dpr = window.devicePixelRatio || 1;
      const x = event.payload.position.x / dpr;
      const y = event.payload.position.y / dpr;
      if (rect && (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom)) {
        return; // dropped over a different pane
      }
      const quoted = event.payload.paths.map(shellQuotePath).join(" ");
      if (quoted) cfg.writeInput(quoted);
    });
  }

  return () => {
    dropOff?.();
    for (const d of disposables) d.dispose();
    search.dispose();
    webLinks.dispose();
    term.options.linkHandler = null;
    clearActiveTerminal(cfg.controllerRef.current);
    cfg.searchRef.current = null;
  };
}

export interface TerminalPaneExtrasOptions {
  sessionId: string | null;
  sessionLabel?: string | null;
  getSurfaceId: () => string | null;
  writeInput: (text: string) => void;
  containerRef: React.RefObject<HTMLDivElement | null>;
  isDesktop?: boolean;
}

export interface TerminalPaneExtras {
  hasSelection: boolean;
  overlay: React.ReactNode;
  onPointerDownCapture: () => void;
  // Attach addons + handlers to a mounted terminal; returns a disposer once the addons load. Call
  // inside an effect via the subscribe() bridge once the Terminal exists.
  attach: (term: Terminal, fitAddon: FitAddon) => Promise<() => void>;
}

export function useTerminalPaneExtras(opts: TerminalPaneExtrasOptions): TerminalPaneExtras {
  const { sessionId, getSurfaceId, writeInput, containerRef } = opts;
  const isDesktop = opts.isDesktop ?? isDesktopHost();

  const termRef = React.useRef<Terminal | null>(null);
  const fitRef = React.useRef<FitAddon | null>(null);
  const searchRef = React.useRef<SearchAddon | null>(null);

  const [searchOpen, setSearchOpen] = React.useState(false);
  const [searchSeed, setSearchSeed] = React.useState<{ query: string; nonce: number }>({
    query: "",
    nonce: 0,
  });
  const [results, setResults] = React.useState<TerminalSearchResults | null>(null);
  const [hasSelection, setHasSelection] = React.useState(false);
  const [pastePreview, setPastePreview] = React.useState<string | null>(null);

  const { copyOnSelect, confirmPaste } = useTerminalClipboardSettings();
  const copyOnSelectRef = React.useRef(copyOnSelect);
  copyOnSelectRef.current = copyOnSelect;
  const confirmPasteRef = React.useRef(confirmPaste);
  confirmPasteRef.current = confirmPaste;

  const refit = React.useCallback(() => fitRef.current?.fit(), []);
  const typography = useLiveTerminalTypography(termRef, refit);
  const typoRef = React.useRef(typography);
  typoRef.current = typography;

  const openFind = React.useCallback((query = "") => {
    setSearchSeed((s) => ({ query, nonce: s.nonce + 1 }));
    setSearchOpen(true);
  }, []);

  const controllerRef = React.useRef<TerminalController>({
    openFind,
    copySelection: () => {},
    paste: () => {},
    selectAll: () => {},
    clear: () => {},
    searchSelection: () => {},
  });
  controllerRef.current = {
    openFind,
    copySelection: () => {
      const sel = termRef.current?.getSelection();
      if (sel) void navigator.clipboard.writeText(sel);
    },
    paste: () => {
      if (termRef.current)
        pasteFromClipboard(termRef.current, confirmPasteRef.current, setPastePreview);
    },
    selectAll: () => termRef.current?.selectAll(),
    clear: () => termRef.current?.clear(),
    searchSelection: () => {
      const sel = termRef.current?.getSelection();
      openFind(sel && sel.length > 0 ? sel : "");
    },
  };

  const onPointerDownCapture = React.useCallback(() => {
    setActiveTerminal(controllerRef.current);
  }, []);

  const searchController = React.useMemo<TerminalSearchController>(
    () => ({
      findNext: (q, o) => searchRef.current?.findNext(q, toSearchOptions(o)),
      findPrevious: (q, o) => searchRef.current?.findPrevious(q, toSearchOptions(o)),
      clear: () => searchRef.current?.clearDecorations(),
    }),
    [],
  );

  const closeSearch = React.useCallback(() => {
    setSearchOpen(false);
    setResults(null);
    searchRef.current?.clearDecorations();
    termRef.current?.focus();
  }, []);

  const cfgRef = React.useRef<AttachConfig | null>(null);
  cfgRef.current = {
    termRef,
    fitRef,
    searchRef,
    controllerRef,
    typoRef,
    copyOnSelectRef,
    containerRef,
    writeInput,
    sessionId,
    sessionLabel: opts.sessionLabel ?? null,
    getSurfaceId,
    isDesktop,
    setResults,
    setHasSelection,
  };

  const attach = React.useCallback(
    (term: Terminal, fitAddon: FitAddon): Promise<() => void> =>
      attachTerminalExtras(term, fitAddon, cfgRef.current as AttachConfig),
    [],
  );

  const overlay = (
    <>
      {searchOpen && (
        <TerminalSearchOverlay
          key={searchSeed.nonce}
          controller={searchController}
          results={results}
          initialQuery={searchSeed.query}
          onClose={closeSearch}
        />
      )}
      <AlertDialog
        open={pastePreview !== null}
        onOpenChange={(open) => !open && setPastePreview(null)}
      >
        <AlertDialogContent data-testid="terminal-paste-confirm">
          <AlertDialogTitle>Paste multiple lines?</AlertDialogTitle>
          <AlertDialogDescription>
            The clipboard contains multiple lines, which may run as separate commands.
          </AlertDialogDescription>
          <pre className="max-h-40 overflow-auto border border-border bg-muted p-2 text-[0.833rem] whitespace-pre-wrap">
            {pastePreview}
          </pre>
          <div className="flex justify-end gap-2">
            <AlertDialogCancel onClick={() => setPastePreview(null)}>Cancel</AlertDialogCancel>
            <AlertDialogAction
              data-testid="terminal-paste-confirm-accept"
              onClick={() => {
                if (pastePreview !== null) termRef.current?.paste(pastePreview);
                setPastePreview(null);
              }}
            >
              Paste
            </AlertDialogAction>
          </div>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );

  return { hasSelection, overlay, onPointerDownCapture, attach };
}
