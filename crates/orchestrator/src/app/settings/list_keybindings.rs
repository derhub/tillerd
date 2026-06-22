use std::collections::HashMap;

use serde::Deserialize;

use crate::app::settings::KeybindingView;
use crate::context::Ctx;
use crate::infra::config::keybinding::KeybindingEntry;
use crate::infra::config::KeybindingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// The effective keymap: defaults merged with user overrides. `defaults_json`
/// carries the compiled-in default keymap as a serialized `{action: chord}` map.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListKeybindings {
    pub defaults_json: String,
}

impl Query<Ctx> for ListKeybindings {
    type Out = Vec<KeybindingView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let defaults: HashMap<String, String> = serde_json::from_str(&self.defaults_json)?;
        let store = KeybindingStore::new(cx.fs_root(), defaults);
        let overrides = store.overrides().await?;

        // Overrides win. A default chord is suppressed when its action is overridden.
        let mut merged: HashMap<String, String> = store.defaults().clone();
        for (action, chord) in overrides {
            merged.insert(action, chord);
        }

        let mut entries: Vec<KeybindingEntry> = merged
            .into_iter()
            .map(|(action, chord)| KeybindingEntry { action, chord })
            .collect();
        entries.sort_by(|a, b| a.action.cmp(&b.action));

        Ok(entries.into_iter().map(KeybindingView).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::rebind_key::RebindKey;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    // Scenario: defaults appear in the list when no overrides exist
    #[tokio::test]
    async fn list_keybindings_includes_defaults() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let entries = bus
            .query(ListKeybindings {
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        let new_sess = entries
            .iter()
            .find(|e| e.0.action == "new-session")
            .unwrap();
        assert_eq!(new_sess.0.chord, "ctrl+n");
    }

    // Scenario: override replaces the default chord for that action
    #[tokio::test]
    async fn list_keybindings_override_wins_over_default() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(RebindKey {
            action: "rename".to_owned(),
            chord: "ctrl+r".to_owned(),
            defaults_json: default_keys_json(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ListKeybindings {
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        let rename = entries.iter().find(|e| e.0.action == "rename").unwrap();
        assert_eq!(rename.0.chord, "ctrl+r");
    }

    // Scenario: result is sorted by action
    #[tokio::test]
    async fn list_keybindings_result_is_sorted_by_action() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let entries = bus
            .query(ListKeybindings {
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        let actions: Vec<&str> = entries.iter().map(|e| e.0.action.as_str()).collect();
        let mut sorted = actions.clone();
        sorted.sort();
        assert_eq!(actions, sorted);
    }

    // Scenario: default chord is suppressed when its action is overridden
    #[tokio::test]
    async fn list_keybindings_default_chord_suppressed_when_action_overridden() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        // Rebind new-session away from ctrl+n.
        bus.execute(RebindKey {
            action: "new-session".to_owned(),
            chord: "ctrl+t".to_owned(),
            defaults_json: default_keys_json(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ListKeybindings {
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        // ctrl+n must not appear anywhere -- the default chord is gone.
        assert!(!entries.iter().any(|e| e.0.chord == "ctrl+n"));
        // ctrl+t must appear for new-session.
        let new_sess = entries
            .iter()
            .find(|e| e.0.action == "new-session")
            .unwrap();
        assert_eq!(new_sess.0.chord, "ctrl+t");
    }
}
