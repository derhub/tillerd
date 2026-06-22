use crate::context::Ctx;
use crate::entities::session::Session;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::Result;

/// Fuzzy-search sessions by title. Filtering evaluated sqlite-side (LIKE).
pub struct SearchSessions {
    pub query: String,
}

impl Query<Ctx> for SearchSessions {
    type Out = Vec<Session>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        SessionRepo::search(cx.db(), &self.query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::new_session_cmd::NewSessionCmd;
    use crate::app::session::test_util::{ctx, draft, unfiled};
    use crate::entities::session::{NewSession, TitleSource};

    // Scenario: Fuzzy search filters in the query
    #[tokio::test]
    async fn search_sessions_returns_matching_by_title() {
        let (bus, _) = ctx().await;

        bus.execute(NewSessionCmd(NewSession {
            project_id: Some(unfiled()),
            title_source: TitleSource::Custom,
            title: Some("alpha session".to_owned()),
            template_id: None,
        }))
        .await
        .unwrap();

        bus.execute(NewSessionCmd(NewSession {
            project_id: Some(unfiled()),
            title_source: TitleSource::Custom,
            title: Some("beta terminal".to_owned()),
            template_id: None,
        }))
        .await
        .unwrap();

        let results = bus
            .query(SearchSessions {
                query: "alpha".to_owned(),
            })
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("alpha"));
    }

    #[tokio::test]
    async fn search_sessions_returns_empty_for_no_match() {
        let (bus, _) = ctx().await;
        bus.execute(NewSessionCmd(draft(unfiled()))).await.unwrap();
        let results = bus
            .query(SearchSessions {
                query: "zzznomatch".to_owned(),
            })
            .await
            .unwrap();
        assert!(results.is_empty());
    }
}
