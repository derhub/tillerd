use serde::Deserialize;

use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Command;

/// Rename a session. Sets `title_source` to `Custom` so automatic titling
/// does not override the user's choice.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameSession {
    pub id: String,
    pub title: String,
}

impl Command<Ctx> for RenameSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let id = SessionId::from_string(&self.id);
        let mut s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;
        s.rename(&self.title);
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};

    // Scenario: Renaming a session marks its title as custom
    #[tokio::test]
    async fn rename_session_sets_title_and_title_source_to_custom() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(RenameSession {
            id: id.clone(),
            title: "Renamed".to_owned(),
        })
        .await
        .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.title, "Renamed");
        assert_eq!(s.title_source, "custom");
    }

    #[tokio::test]
    async fn rename_trims_whitespace() {
        let (bus, _) = ctx().await;
        let id = create_one(&bus).await;

        bus.execute(RenameSession {
            id: id.clone(),
            title: "  spaced  ".to_owned(),
        })
        .await
        .unwrap();

        let s = bus.query(GetSessionById { id }).await.unwrap().unwrap();
        assert_eq!(s.title, "spaced");
    }
}
