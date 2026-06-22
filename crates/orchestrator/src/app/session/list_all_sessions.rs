use serde::Deserialize;
use sqlx::AssertSqlSafe;

use crate::app::session::SessionView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};

/// Projection columns for the `SessionView` read model.
const SELECT: &str = "SELECT id, project_id, title, title_source, created_at FROM session";

/// List every session across all projects, pinned-first. The sidebar groups
/// these by project, so it loads them in one call (no project filter).
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAllSessions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListAllSessions {
    type Out = Listing<SessionView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let page = if self.after.is_some() {
            Page::Cursor {
                after: self.after.clone(),
                limit: self.limit.unwrap_or(0),
            }
        } else if let Some(limit) = self.limit {
            Page::Offset {
                limit,
                offset: self.offset.unwrap_or(0),
            }
        } else {
            Page::All
        };

        let order = "ORDER BY pinned DESC, sort_order ASC, id ASC";
        match page {
            Page::All => {
                let items: Vec<SessionView> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {order}")))
                        .fetch_all(cx.db())
                        .await?;
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let items: Vec<SessionView> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {order} LIMIT ? OFFSET ?")))
                        .bind(limit as i64)
                        .bind(offset as i64)
                        .fetch_all(cx.db())
                        .await?;
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                let fetch_n = limit as i64 + 1;
                let rows: Vec<SessionView> = if let Some(cursor) = after {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "WITH anchor AS (
                             SELECT pinned, sort_order FROM session WHERE id = ?
                         )
                         {SELECT}
                         WHERE (pinned < (SELECT pinned FROM anchor)
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order > (SELECT sort_order FROM anchor))
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order = (SELECT sort_order FROM anchor)
                                    AND id > ?))
                         {order}
                         LIMIT ?"
                    )))
                    .bind(&cursor)
                    .bind(&cursor)
                    .bind(fetch_n)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {order} LIMIT ?")))
                        .bind(fetch_n)
                        .fetch_all(cx.db())
                        .await?
                };
                let has_more = rows.len() > limit as usize;
                let items: Vec<SessionView> = rows.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    items.last().map(|s| s.id.clone())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::test_util::*;

    #[tokio::test]
    async fn list_all_returns_every_session_unfiltered() {
        let (bus, _pool) = ctx().await;
        let _ = create_one(&bus).await;
        let _ = create_one(&bus).await;

        let all = bus
            .query(ListAllSessions {
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(
            all.items.len() >= 2,
            "list_all must return all sessions, got {}",
            all.items.len()
        );
    }
}
