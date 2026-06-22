use std::collections::HashMap;

use crate::context::Ctx;
use crate::infra::config::KeybindingStore;
use crate::shared::cqs::Command;
use crate::shared::Result;

/// Set or override an action's chord. Carries the defaults so the store can
/// merge effectively.
pub struct RebindKey {
    pub action: String,
    pub chord: String,
    pub defaults: HashMap<String, String>,
}

impl Command<Ctx> for RebindKey {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        KeybindingStore::new(cx.fs_root(), self.defaults.clone())
            .rebind(&self.action, &self.chord)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::list_keybindings::ListKeybindings;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn rebind_key_then_list_reflects_override() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        bus.execute(RebindKey {
            action: "rename".to_owned(),
            chord: "ctrl+r".to_owned(),
            defaults: default_keys(),
        })
        .await
        .unwrap();

        let entries = bus
            .query(ListKeybindings {
                defaults: default_keys(),
            })
            .await
            .unwrap();

        let rename = entries.iter().find(|e| e.action == "rename").unwrap();
        assert_eq!(rename.chord, "ctrl+r");
    }
}
