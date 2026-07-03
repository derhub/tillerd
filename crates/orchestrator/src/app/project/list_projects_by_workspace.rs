use serde::Deserialize;

use crate::app::project::common::{make_cursor, parse_cursor};
use crate::app::project::ProjectView;
use crate::context::Ctx;
use crate::shared::pagination::{Listing, Page};
use crate::shared::{Query, Result};

/// List projects in a workspace (pinned-first then by sort_order).
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProjectsByWorkspace {
    pub workspace_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

/// A `ProjectView` plus the two cursor-key columns (`pinned`, `sort_order`) needed
/// to mint the continuation cursor without re-querying.
#[derive(sqlx::FromRow)]
struct CursorRow {
    #[sqlx(flatten)]
    view: ProjectView,
    pinned: i64,
    sort_order: i64,
}

impl Query<Ctx> for ListProjectsByWorkspace {
    type Out = Listing<ProjectView>;
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

        match page {
            Page::All => {
                let items = sqlx::query_as::<_, ProjectView>(
                    "SELECT id, name, source_kind, root_path, workspace_id,
                            CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
                     FROM project
                     WHERE workspace_id = ?
                     ORDER BY pinned DESC, sort_order, id",
                )
                .bind(&self.workspace_id)
                .fetch_all(cx.db())
                .await?;
                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                // Fetch one extra to detect whether a next page exists.
                let fetch = (limit as i64) + 1;
                let rows = sqlx::query_as::<_, ProjectView>(
                    "SELECT id, name, source_kind, root_path, workspace_id,
                            CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status
                     FROM project
                     WHERE workspace_id = ?
                     ORDER BY pinned DESC, sort_order, id
                     LIMIT ? OFFSET ?",
                )
                .bind(&self.workspace_id)
                .bind(fetch)
                .bind(offset as i64)
                .fetch_all(cx.db())
                .await?;

                let has_more = rows.len() > limit as usize;
                let items: Vec<ProjectView> = rows.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    let new_offset = offset + limit;
                    Some(format!("offset:{new_offset}"))
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                // Cursor format: "{pinned}:{sort_order}:{id}"
                // Rows strictly after the cursor position in (pinned DESC, sort_order, id) order.
                // `pinned`/`sort_order` ride along to mint the next cursor.
                let fetch = (limit as i64) + 1;
                let rows: Vec<CursorRow> = if let Some(cursor) = &after {
                    let (c_pinned, c_sort, c_id) = parse_cursor(cursor)?;
                    sqlx::query_as(
                        "SELECT id, name, source_kind, root_path, workspace_id,
                                CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status,
                                pinned, sort_order
                         FROM project
                         WHERE workspace_id = ?
                           AND (
                             pinned < ?
                             OR (pinned = ? AND sort_order > ?)
                             OR (pinned = ? AND sort_order = ? AND id > ?)
                           )
                         ORDER BY pinned DESC, sort_order, id
                         LIMIT ?",
                    )
                    .bind(&self.workspace_id)
                    .bind(c_pinned)
                    .bind(c_pinned)
                    .bind(c_sort)
                    .bind(c_pinned)
                    .bind(c_sort)
                    .bind(&c_id)
                    .bind(fetch)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as(
                        "SELECT id, name, source_kind, root_path, workspace_id,
                                CASE WHEN archived_at IS NOT NULL THEN 'archived' ELSE 'active' END AS status,
                                pinned, sort_order
                         FROM project
                         WHERE workspace_id = ?
                         ORDER BY pinned DESC, sort_order, id
                         LIMIT ?",
                    )
                    .bind(&self.workspace_id)
                    .bind(fetch)
                    .fetch_all(cx.db())
                    .await?
                };

                let has_more = rows.len() > limit as usize;
                let page_rows: Vec<CursorRow> = rows.into_iter().take(limit as usize).collect();

                let next = if has_more {
                    page_rows
                        .last()
                        .map(|r| make_cursor(r.pinned != 0, r.sort_order as u32, &r.view.id))
                } else {
                    None
                };

                let items: Vec<ProjectView> = page_rows.into_iter().map(|r| r.view).collect();
                Ok(Listing::new(items, next))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;
    use crate::entities::workspace::WorkspaceId;

    async fn seed_workspace(pool: &sqlx::SqlitePool, ws_id: &str) {
        sqlx::query("INSERT INTO workspace (id, name) VALUES (?, ?)")
            .bind(ws_id)
            .bind("Other")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn list_projects_by_workspace_does_not_mutate() {
        let (_ctx, bus) = ctx().await;
        // Query returns listing; no write occurs.
        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        // The seed Unfiled project is always present.
        assert!(!listing.items.is_empty());
    }

    #[tokio::test]
    async fn list_projects_by_workspace_filters_correctly() {
        let (ctx, bus) = ctx().await;

        // Seed a second workspace.
        let other_ws_id = "ws-other-test-0001";
        seed_workspace(ctx.db(), other_ws_id).await;

        seed_project(ctx.db(), "p-ws1", "In Default", &default_ws()).await;
        seed_project(
            ctx.db(),
            "p-ws2",
            "In Other",
            &WorkspaceId::new(other_ws_id),
        )
        .await;

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        let ids: Vec<&str> = listing.items.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"p-ws1"), "own project must appear");
        assert!(!ids.contains(&"p-ws2"), "other-ws project must not appear");
    }

    #[tokio::test]
    async fn list_returns_pinned_before_unpinned() {
        let (ctx, bus) = ctx().await;

        // sort_order: pinned=20, unpinned=10 -- pinned still comes first.
        seed_project_full(ctx.db(), "p-unpinned", "Unpinned", &default_ws(), 10).await;
        seed_project_full(ctx.db(), "p-pinned", "Pinned", &default_ws(), 20).await;
        sqlx::query("UPDATE project SET pinned = 1 WHERE id = ?")
            .bind("p-pinned")
            .execute(ctx.db())
            .await
            .unwrap();

        let listing = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        // Filter out the seeded Unfiled project.
        let names: Vec<&str> = listing
            .items
            .iter()
            .filter(|p| p.id != "00000000-0000-0000-0000-000000000000")
            .map(|p| p.name.as_str())
            .collect();

        assert_eq!(names, vec!["Pinned", "Unpinned"]);
    }

    #[tokio::test]
    async fn offset_pagination_returns_cursor_when_more_remain() {
        let (ctx, bus) = ctx().await;

        for i in 0u32..4 {
            seed_project_full(
                ctx.db(),
                &format!("p-page-{i:02}"),
                &format!("Page {i}"),
                &default_ws(),
                i,
            )
            .await;
        }

        let page1 = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: Some(2),
                offset: Some(0),
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(page1.items.len(), 2);
        assert!(page1.next.is_some(), "must carry a continuation cursor");
    }

    #[tokio::test]
    async fn offset_last_page_has_no_next_cursor() {
        let (ctx, bus) = ctx().await;

        for i in 0u32..2 {
            seed_project_full(
                ctx.db(),
                &format!("p-last-{i:02}"),
                &format!("Last {i}"),
                &default_ws(),
                i,
            )
            .await;
        }

        // 3 total rows (2 new + 1 Unfiled seed) with limit=10 -> fits on one page.
        let page = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: Some(10),
                offset: Some(0),
                after: None,
            })
            .await
            .unwrap();
        assert!(page.next.is_none(), "no next cursor on last page");
    }

    #[tokio::test]
    async fn cursor_pagination_pages_through_all_items() {
        let (ctx, bus) = ctx().await;

        for i in 0u32..3 {
            // avoid collision with seed Unfiled sort_order=0
            seed_project_full(
                ctx.db(),
                &format!("p-cursor-{i:02}"),
                &format!("Cursor {i}"),
                &default_ws(),
                i + 100,
            )
            .await;
        }

        // Drive cursor mode (after.is_some()) starting just before the seeded
        // unpinned rows (the Unfiled seed sits at pinned=false, sort_order=0).
        let start = make_cursor(false, 0, "00000000-0000-0000-0000-000000000000");
        let p1 = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: Some(2),
                offset: None,
                after: Some(start),
            })
            .await
            .unwrap();
        assert_eq!(p1.items.len(), 2);
        let cursor = p1.next.clone().expect("must have next cursor");

        let p2 = bus
            .query(ListProjectsByWorkspace {
                workspace_id: default_ws().as_str().to_owned(),
                limit: Some(2),
                offset: None,
                after: Some(cursor),
            })
            .await
            .unwrap();
        // 3 seeded rows after the start cursor; first page took 2, second page has 1.
        assert_eq!(p2.items.len(), 1);

        // No overlap.
        let p1_ids: Vec<&str> = p1.items.iter().map(|p| p.id.as_str()).collect();
        let p2_ids: Vec<&str> = p2.items.iter().map(|p| p.id.as_str()).collect();
        for id in &p2_ids {
            assert!(!p1_ids.contains(id), "pages must not overlap");
        }
    }
}
