use serde::Deserialize;
use sqlx::AssertSqlSafe;

use crate::app::session::common::PINNED_FIRST;
use crate::app::session::SessionView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};

/// Projection columns for the `SessionView` read model.
const SELECT: &str = "SELECT id, project_id, title, title_source, created_at, pinned,
                             CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
                      FROM session";

/// List sessions in a project, pinned-first.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSessionsByProject {
    pub project_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListSessionsByProject {
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

        let order = PINNED_FIRST;
        let where_parent = "WHERE project_id = ?";

        match page {
            Page::All => {
                let items: Vec<SessionView> =
                    sqlx::query_as(AssertSqlSafe(format!("{SELECT} {where_parent} {order}")))
                        .bind(&self.project_id)
                        .fetch_all(cx.db())
                        .await?;
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let items: Vec<SessionView> = sqlx::query_as(AssertSqlSafe(format!(
                    "{SELECT} {where_parent} {order} LIMIT ? OFFSET ?"
                )))
                .bind(&self.project_id)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(cx.db())
                .await?;
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                // Fetch limit+1 to detect whether a next page exists without a
                // COUNT query. More than limit rows back means a next page
                // exists; truncate to limit before returning.
                let fetch_n = limit as i64 + 1;
                let rows: Vec<SessionView> = if let Some(cursor) = after {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "WITH anchor AS (
                             SELECT pinned, sort_order FROM session WHERE id = ?
                         )
                         {SELECT} {where_parent}
                           AND (pinned < (SELECT pinned FROM anchor)
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order > (SELECT sort_order FROM anchor))
                                OR (pinned = (SELECT pinned FROM anchor)
                                    AND sort_order = (SELECT sort_order FROM anchor)
                                    AND id > ?))
                         {order}
                         LIMIT ?"
                    )))
                    .bind(&cursor)
                    .bind(&self.project_id)
                    .bind(&cursor)
                    .bind(fetch_n)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{SELECT} {where_parent} {order} LIMIT ?"
                    )))
                    .bind(&self.project_id)
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
    use crate::entities::project::ProjectId;
    use crate::entities::session::{Session, SessionId, SessionStatus, TitleSource};
    use crate::infra::session::SessionRepo;

    fn make_session(id: &str, project_id: &ProjectId, sort_order: u32) -> Session {
        Session {
            id: SessionId::from_string(id),
            project_id: project_id.clone(),
            title: format!("Session {id}"),
            title_source: TitleSource::AgentTitle,
            created_at: "2026-01-01T00:00:00.000Z".to_owned(),
            spec_version: None,
            spec_json: None,
            sort_order,
            pinned: false,
            status: SessionStatus::Active,
        }
    }

    // Scenario: Children are found by parent id
    #[tokio::test]
    async fn list_filters_by_project_id() {
        let (bus, pool) = ctx().await;
        let unfiled_pid = unfiled();

        // Insert a second project under the seeded Default workspace.
        sqlx::query("INSERT INTO project (id, workspace_id, name) VALUES (?, ?, ?)")
            .bind("proj-other")
            .bind("00000000-0000-0000-0000-000000000001")
            .bind("Other")
            .execute(&pool)
            .await
            .unwrap();

        let other_pid = ProjectId::new("proj-other");

        SessionRepo::create(&pool, &make_session("s-f-1", &unfiled_pid, 0))
            .await
            .unwrap();
        SessionRepo::create(&pool, &make_session("s-f-2", &unfiled_pid, 1))
            .await
            .unwrap();
        SessionRepo::create(&pool, &make_session("s-f-other", &other_pid, 0))
            .await
            .unwrap();

        let listing = bus
            .query(ListSessionsByProject {
                project_id: unfiled_pid.as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|s| s.id.as_str()).collect();

        assert!(ids.contains(&"s-f-1"));
        assert!(ids.contains(&"s-f-2"));
        assert!(!ids.contains(&"s-f-other"));
    }

    // Scenario: A pinned item sorts ahead of unpinned
    #[tokio::test]
    async fn list_returns_pinned_first() {
        let (bus, pool) = ctx().await;
        let pid = unfiled();

        let unpinned = make_session("s-unpinned", &pid, 0);
        let pinned = Session {
            pinned: true,
            sort_order: 99, // high sort_order; pinned flag dominates
            ..make_session("s-pinned", &pid, 99)
        };

        SessionRepo::create(&pool, &unpinned).await.unwrap();
        SessionRepo::create(&pool, &pinned).await.unwrap();

        let listing = bus
            .query(ListSessionsByProject {
                project_id: pid.as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(listing.items[0].id, "s-pinned");
        assert_eq!(listing.items[1].id, "s-unpinned");
    }

    // Scenario: Offset pagination
    #[tokio::test]
    async fn list_offset_respects_limit_and_offset() {
        let (bus, pool) = ctx().await;
        let pid = unfiled();
        for i in 0u32..5 {
            SessionRepo::create(&pool, &make_session(&format!("s-off-{i}"), &pid, i))
                .await
                .unwrap();
        }

        let page1 = bus
            .query(ListSessionsByProject {
                project_id: pid.as_str().to_owned(),
                limit: Some(2),
                offset: Some(0),
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert_eq!(page1.items[0].id, "s-off-0");
        assert_eq!(page1.items[1].id, "s-off-1");

        let page2 = bus
            .query(ListSessionsByProject {
                project_id: pid.as_str().to_owned(),
                limit: Some(2),
                offset: Some(2),
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 2);
        assert_eq!(page2.items[0].id, "s-off-2");
        assert_eq!(page2.items[1].id, "s-off-3");
    }

    // Scenario: A bounded cursor page returns a continuation cursor.
    // Cursor mode is driven by `after`; anchors on the first row, then pages.
    #[tokio::test]
    async fn list_cursor_returns_next_when_more_remain() {
        let (bus, pool) = ctx().await;
        let pid = unfiled();
        for i in 0u32..4 {
            SessionRepo::create(&pool, &make_session(&format!("s-cur-{i}"), &pid, i))
                .await
                .unwrap();
        }

        // Anchor on the first row (sort_order 0) and request 2 of the 3 remaining.
        let page1 = bus
            .query(ListSessionsByProject {
                project_id: pid.as_str().to_owned(),
                limit: Some(2),
                offset: None,
                after: Some("s-cur-0".to_owned()),
            })
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next.is_some(), "should have a continuation cursor");

        let cursor = page1.next.unwrap();
        let page2 = bus
            .query(ListSessionsByProject {
                project_id: pid.as_str().to_owned(),
                limit: Some(2),
                offset: None,
                after: Some(cursor),
            })
            .await
            .unwrap();
        assert_eq!(page2.items.len(), 1);
        assert!(page2.next.is_none(), "last page has no cursor");
    }

    #[tokio::test]
    async fn list_cursor_last_page_has_no_next() {
        let (bus, pool) = ctx().await;
        let pid = unfiled();
        for i in 0u32..3 {
            SessionRepo::create(&pool, &make_session(&format!("s-last-{i}"), &pid, i))
                .await
                .unwrap();
        }

        // Anchor on the first row, large limit -> remaining rows fit on one page.
        let listing = bus
            .query(ListSessionsByProject {
                project_id: pid.as_str().to_owned(),
                limit: Some(10),
                offset: None,
                after: Some("s-last-0".to_owned()),
            })
            .await
            .unwrap();
        assert!(listing.next.is_none());
    }
}
