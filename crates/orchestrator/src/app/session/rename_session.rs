use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Command;
use crate::shared::errors::{Error, Result};

/// Rename a session. Sets `title_source` to `Custom` so automatic titling
/// does not override the user's choice.
pub struct RenameSession {
    pub id: SessionId,
    pub title: String,
}

impl Command<Ctx> for RenameSession {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let mut s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;
        s.rename(&self.title);
        SessionRepo::update(cx.db(), &s).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::get_session_by_id::GetSessionById;
    use crate::app::session::test_util::{create_one, ctx};
    use crate::entities::session::TitleSource;

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
        assert_eq!(s.title_source, TitleSource::Custom);
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
