use serde::Deserialize;

use crate::app::session::SessionView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;

/// Fetch one session by id.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSessionById {
    pub id: String,
}

impl Query<Ctx> for GetSessionById {
    type Out = Option<SessionView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, SessionView>(
            "SELECT id, project_id, title, title_source, created_at,
                    CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
             FROM session
             WHERE id = ?",
        )
        .bind(&self.id)
        .fetch_optional(cx.db())
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::test_util::ctx;

    // Scenario: A query reads and does not mutate
    #[tokio::test]
    async fn get_session_by_id_returns_none_for_missing() {
        let (bus, _) = ctx().await;
        let result = bus
            .query(GetSessionById {
                id: "no-such".to_owned(),
            })
            .await
            .unwrap();
        assert!(result.is_none());
    }
}
