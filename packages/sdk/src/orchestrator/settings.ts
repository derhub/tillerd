// -- command names -------------------------------------------------------------

export const SETTING_GET = "setting_get";
export const SETTING_SET = "setting_set";
export const SETTING_LIST = "setting_list";

// -- domain types --------------------------------------------------------------

/** Scope a setting is stored under: app-global, or bound to a specific project. */
export type SettingScope = "global" | "project";

/** A stored setting: its key and decoded JSON value. */
export interface SettingEntry {
  key: string;
  value: unknown;
}

// -- request shapes --------------------------------------------------------------

export interface GetSettingArgs {
  scope: SettingScope;
  /** Required when `scope` is `"project"`; ignored for `"global"`. */
  projectId?: string;
  key: string;
}

export interface SetSettingArgs {
  scope: SettingScope;
  projectId?: string;
  key: string;
  /** Any JSON-serializable value. */
  value: unknown;
}

export interface ListSettingsArgs {
  scope: SettingScope;
  projectId?: string;
}

// -- client interface ----------------------------------------------------------

/**
 * Host-agnostic settings access. The value is an opaque JSON value; callers narrow
 * it. `getSetting` resolves to `null` when the key is unset under the given scope.
 */
export interface SettingsClient {
  getSetting(args: GetSettingArgs): Promise<unknown>;
  setSetting(args: SetSettingArgs): Promise<void>;
  listSettings(args: ListSettingsArgs): Promise<SettingEntry[]>;
}

export interface SettingsTransport {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

export function createSettingsClient(transport: SettingsTransport): SettingsClient {
  const call = <T>(cmd: string, args?: unknown) =>
    transport.invoke<T>(cmd, args as Record<string, unknown>);
  return {
    getSetting: (args) => call<unknown>(SETTING_GET, args),
    setSetting: (args) => call<void>(SETTING_SET, args),
    listSettings: (args) => call<SettingEntry[]>(SETTING_LIST, args),
  };
}
