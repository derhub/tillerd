import { useNavigate, useSearch } from "@tanstack/react-router";

import { AppearanceSection } from "~/components/settings/AppearanceSection";
import { GeneralSection } from "~/components/settings/GeneralSection";
import { KeybindingSettings } from "~/components/settings/KeybindingSettings";
import { ProfilesSection } from "~/components/settings/ProfilesSection";
import { ProjectSection } from "~/components/settings/ProjectSection";
import { TerminalSection } from "~/components/settings/TerminalSection";
import { ThemesSection } from "~/components/settings/ThemesSection";
import { useActiveProject } from "~/lib/store";
import { cn } from "~/lib/utils";

// Order mirrors the ui-settings-editor spec's own listing: General, Appearance, Terminal,
// Keybindings, Profiles, Themes, and -- only with an active project -- Project.
const BASE_SECTIONS = [
  { id: "general", label: "General" },
  { id: "appearance", label: "Appearance" },
  { id: "terminal", label: "Terminal" },
  { id: "keybindings", label: "Keybindings" },
  { id: "profiles", label: "Profiles" },
  { id: "themes", label: "Themes" },
] as const;

const PROJECT_SECTION = { id: "project", label: "Project" } as const;

type SectionId = (typeof BASE_SECTIONS)[number]["id"] | typeof PROJECT_SECTION.id;

function isSectionId(
  value: string | undefined,
  sections: readonly { id: string }[],
): value is SectionId {
  return sections.some((s) => s.id === value);
}

// The settings editor renders in the panel area (a route, not a popover -- see
// ui-settings-editor spec). Section state rides the `?section=` search param (same
// passthrough idiom as /logs?service=) so it is deep-linkable and switching sections
// is a client-side nav, never a full reload.
export function SettingsEditor() {
  const navigate = useNavigate();
  const activeProjectId = useActiveProject();
  const sections = activeProjectId ? [...BASE_SECTIONS, PROJECT_SECTION] : BASE_SECTIONS;

  const raw = useSearch({ from: "/settings", select: (s) => s.section });
  const active: SectionId = isSectionId(raw, sections) ? raw : "appearance";

  const select = (id: SectionId) =>
    void navigate({ to: "/settings", search: (prev) => ({ ...prev, section: id }) });

  return (
    <div className="h-full w-full min-w-0 flex" data-testid="settings-editor">
      <nav
        aria-label="Settings sections"
        className="w-40 shrink-0 border-r border-border/40 p-2 flex flex-col gap-px overflow-y-auto"
      >
        {sections.map((s) => (
          <button
            key={s.id}
            type="button"
            aria-current={active === s.id ? "page" : undefined}
            data-testid={`settings-section-${s.id}`}
            onClick={() => select(s.id)}
            className={cn(
              "text-left px-2 h-7 rounded-sm text-[0.917rem] transition-colors duration-[var(--motion-fast)] ease-standard",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
              active === s.id
                ? "bg-muted text-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
          >
            {s.label}
          </button>
        ))}
      </nav>
      <div className="flex-1 min-w-0 overflow-y-auto p-4">
        {active === "general" && <GeneralSection />}
        {active === "appearance" && <AppearanceSection />}
        {active === "terminal" && <TerminalSection />}
        {active === "keybindings" && (
          <section aria-label="Keybindings">
            <KeybindingSettings />
          </section>
        )}
        {active === "profiles" && <ProfilesSection />}
        {active === "themes" && <ThemesSection />}
        {active === "project" && activeProjectId && <ProjectSection />}
      </div>
    </div>
  );
}
