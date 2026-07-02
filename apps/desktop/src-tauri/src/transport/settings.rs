use orchestrator::app::settings::{
    ActivateProfile, ActivateTheme, ApplySetting, DiscardProfile, DiscardTheme, DuplicateProfile,
    ExportProfile, ExportTheme, GetActiveProfile, GetActiveTheme, GetProfile, GetSetting,
    ImportProfile, ImportTheme, KeybindingView, ListKeybindings, ListProfiles, ListSettings,
    ListThemes, NewProfile, ProfileView, RebindKey, ReloadConfig, RenameProfile, ResetKeybinding,
    ResetKeybindings, ResetSetting, ResolveKeybinding, ResolveSetting, ResolveSettings,
    SettingView, ThemeView,
};

use crate::transport::macros::{transport_command, transport_create, transport_query};

transport_query!(
    setting_get(scope: String, project_id: Option<String>, key: String) -> Option<String>
        => GetSetting { scope, project_id, key },
        |raw| raw
);

transport_command!(
    setting_set(scope: String, project_id: Option<String>, key: String, value_json: String)
        => ApplySetting { scope, project_id, key, value_json }
);

transport_query!(
    setting_list(scope: String, project_id: Option<String>) -> Vec<SettingView>
        => ListSettings { scope, project_id },
        |settings| settings
);

transport_command!(
    setting_reset(scope: String, project_id: Option<String>, key: String)
        => ResetSetting { scope, project_id, key }
);

transport_query!(
    setting_resolve(project_id: String, key: String) -> Option<String>
        => ResolveSetting { project_id, key },
        |raw| raw
);

transport_query!(
    settings_resolve(project_id: String) -> Vec<SettingView>
        => ResolveSettings { project_id },
        |settings| settings
);

transport_query!(
    profile_get_active() -> Option<ProfileView>
        => GetActiveProfile,
        |profile| profile
);

transport_query!(
    profile_list() -> Vec<ProfileView>
        => ListProfiles,
        |profiles| profiles
);

transport_create!(
    /// Create a new profile with a caller-supplied id (client-assigned identity).
    profile_create(id: String, name: String) -> ProfileView {
        let created = id;
        execute: NewProfile {
            id: created.clone(),
            name,
        },
        read_back: GetProfile { id: created },
        map: |p| p,
        missing: "profile vanished after create",
    }
);

transport_command!(profile_activate(id: String) => ActivateProfile { id });

transport_command!(profile_rename(id: String, new_name: String) => RenameProfile { id, new_name });

transport_command!(
    profile_duplicate(source_id: String, new_id: String, new_name: String)
        => DuplicateProfile { source_id, new_id, new_name }
);

transport_command!(profile_discard(id: String) => DiscardProfile { id });

transport_query!(
    profile_export(id: String) -> Option<Vec<u8>>
        => ExportProfile { id },
        |bytes| bytes
);

transport_command!(profile_import(profile_json: String) => ImportProfile { profile_json });

transport_query!(
    theme_get_active() -> Option<ThemeView>
        => GetActiveTheme,
        |theme| theme
);

transport_query!(
    theme_list() -> Vec<ThemeView>
        => ListThemes,
        |themes| themes
);

transport_command!(theme_activate(id: String) => ActivateTheme { id });

transport_command!(theme_discard(id: String) => DiscardTheme { id });

transport_query!(
    theme_export(id: String) -> Option<Vec<u8>>
        => ExportTheme { id },
        |bytes| bytes
);

transport_command!(
    theme_import(id: String, name: String, origin: String, data_json: Option<String>)
        => ImportTheme { id, name, origin, data_json }
);

transport_query!(
    keybinding_list(defaults_json: String) -> Vec<KeybindingView>
        => ListKeybindings { defaults_json },
        |entries| entries
);

transport_command!(
    keybinding_rebind(action: String, chord: String, defaults_json: String)
        => RebindKey { action, chord, defaults_json }
);

transport_command!(
    keybinding_reset(action: String, defaults_json: String)
        => ResetKeybinding { action, defaults_json }
);

transport_command!(
    keybinding_reset_all(defaults_json: String)
        => ResetKeybindings { defaults_json }
);

transport_query!(
    keybinding_resolve(action: String, defaults_json: String) -> Option<String>
        => ResolveKeybinding { action, defaults_json },
        |chord| chord
);

transport_command!(config_reload() => ReloadConfig);

#[cfg(test)]
mod tests {

    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn profile_response_matches_sdk_profile_shape() {
        let raw = serde_json::json!({ "id": "p", "name": "P", "settings": {} });
        assert_keys(&raw, &["id", "name", "settings"]);
    }

    #[test]
    fn theme_response_matches_sdk_theme_shape() {
        let raw =
            serde_json::json!({ "id": "t", "name": "T", "origin": "custom", "data_json": null });
        assert_keys(&raw, &["id", "name", "origin", "data_json"]);
    }

    #[test]
    fn keybinding_response_matches_sdk_keybinding_shape() {
        let raw = serde_json::json!({ "action": "rename", "chord": "F2" });
        assert_keys(&raw, &["action", "chord"]);
    }
}
