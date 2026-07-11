// Settings-open signal. Decoupled from the editor's location so any trigger (status bar,
// command palette, native menu, e2e harness) can request the settings editor without
// importing the route/component directly. Historically opened a popover; now navigates
// to the `/settings` route -- the event id is a load-bearing e2e/palette contract, kept
// stable across that migration.
export const SETTINGS_OPEN_EVENT = "command-center:settings";
