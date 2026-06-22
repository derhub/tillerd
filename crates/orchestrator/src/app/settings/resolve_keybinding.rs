use std::collections::HashMap;

use serde::Deserialize;

use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// The chord bound to an action (or `None` if unbound). `defaults_json` carries
/// the default keymap as a serialized `{action: chord}` map.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveKeybinding {
    pub action: String,
    pub defaults_json: String,
}

impl Query<Ctx> for ResolveKeybinding {
    type Out = Option<String>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let defaults: HashMap<String, String> = serde_json::from_str(&self.defaults_json)?;
        KeybindingStore::new(cx.fs_root(), defaults)
            .resolve(&self.action)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::rebind_key::RebindKey;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn resolve_keybinding_returns_override_over_default() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(RebindKey {
            action: "rename".to_owned(),
            chord: "ctrl+shift+r".to_owned(),
            defaults_json: default_keys_json(),
        })
        .await
        .unwrap();

        let chord = bus
            .query(ResolveKeybinding {
                action: "rename".to_owned(),
                defaults_json: default_keys_json(),
            })
            .await
            .unwrap();

        assert_eq!(chord.as_deref(), Some("ctrl+shift+r"));
    }
}
