use std::collections::HashMap;

use crate::context::Ctx;
use crate::infra::config::keybinding::KeybindingEntry;
use crate::infra::config::KeybindingStore;
use crate::shared::cqs::Query;
use crate::shared::Result;

/// The effective keymap: defaults merged with user overrides.
pub struct ListKeybindings {
    pub defaults: HashMap<String, String>,
}

impl Query<Ctx> for ListKeybindings {
    type Out = Vec<KeybindingEntry>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        KeybindingStore::new(cx.fs_root(), self.defaults.clone())
            .list()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::settings::test_util::*;
    use crate::shared::bus::Bus;

    #[tokio::test]
    async fn list_keybindings_includes_defaults() {
        let dir = TempDir::new().unwrap();
        let bus = Bus::new(make_ctx(&dir).await);

        let entries = bus
            .query(ListKeybindings {
                defaults: default_keys(),
            })
            .await
            .unwrap();

        let new_sess = entries.iter().find(|e| e.action == "new-session").unwrap();
        assert_eq!(new_sess.chord, "ctrl+n");
    }
}
