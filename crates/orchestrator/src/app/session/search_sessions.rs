use serde::Deserialize;

use crate::app::session::SessionView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;

/// Fuzzy-search sessions by title. Filtering evaluated sqlite-side (LIKE).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSessions {
    pub query: String,
}

impl Query<Ctx> for SearchSessions {
    type Out = Vec<SessionView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let pattern = format!("%{}%", self.query);
        Ok(sqlx::query_as::<_, SessionView>(
            "SELECT id, project_id, title, title_source, created_at,
                    CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
             FROM session
             WHERE title LIKE ?
             ORDER BY pinned DESC, sort_order ASC, id ASC",
        )
        .bind(pattern)
        .fetch_all(cx.db())
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::new_session_cmd::NewSessionCmd;
    use crate::app::session::test_util::{ctx, unfiled};
    // Scenario: Fuzzy search filters in the query
    #[tokio::test]
    async fn search_sessions_returns_matching_by_title() {
        let (bus, _) = ctx().await;

        bus.execute(NewSessionCmd {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: Some(unfiled().as_str().to_owned()),
            title_source: "custom".to_owned(),
            title: Some("alpha session".to_owned()),
            template_id: None,
        })
        .await
        .unwrap();

        bus.execute(NewSessionCmd {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: Some(unfiled().as_str().to_owned()),
            title_source: "custom".to_owned(),
            title: Some("beta terminal".to_owned()),
            template_id: None,
        })
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
        use crate::app::session::test_util::create_one;
        let (bus, _) = ctx().await;
        let _ = create_one(&bus).await;
        let results = bus
            .query(SearchSessions {
                query: "zzznomatch".to_owned(),
            })
            .await
            .unwrap();
        assert!(results.is_empty());
    }
}
