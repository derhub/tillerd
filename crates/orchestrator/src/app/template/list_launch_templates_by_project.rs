use serde::Deserialize;

use crate::app::template::LaunchTemplateView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// List a project's launch templates, ordered by `sort_order` then `id`.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListLaunchTemplatesByProject {
    pub project_id: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListLaunchTemplatesByProject {
    type Out = Listing<LaunchTemplateView>;

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

        let pid = self.project_id.as_str();

        match page {
            Page::All => {
                let items = sqlx::query_as::<_, LaunchTemplateView>(
                    "SELECT id, project_id, spec_version, spec_json
                     FROM launch_template
                     WHERE project_id = ?
                     ORDER BY sort_order ASC, id ASC",
                )
                .bind(pid)
                .fetch_all(cx.db())
                .await?;

                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                let limit = limit as i64;
                let offset = offset as i64;
                // Fetch limit+1 to detect whether a next page exists.
                let mut rows = sqlx::query_as::<_, LaunchTemplateView>(
                    "SELECT id, project_id, spec_version, spec_json
                     FROM launch_template
                     WHERE project_id = ?
                     ORDER BY sort_order ASC, id ASC
                     LIMIT ? OFFSET ?",
                )
                .bind(pid)
                .bind(limit + 1)
                .bind(offset)
                .fetch_all(cx.db())
                .await?;

                let has_more = rows.len() as i64 > limit;
                if has_more {
                    rows.truncate(limit as usize);
                }
                let next = if has_more {
                    Some(format!("{}", offset + limit))
                } else {
                    None
                };

                Ok(Listing::new(rows, next))
            }

            Page::Cursor { after, limit } => {
                let limit = limit as i64;

                let mut rows = if let Some(cursor) = after {
                    // cursor is the id of the last-seen row; use (sort_order, id) for stable ordering
                    sqlx::query_as::<_, LaunchTemplateView>(
                        "SELECT t.id, t.project_id, t.spec_version, t.spec_json
                         FROM launch_template t
                         WHERE t.project_id = ?
                           AND (t.sort_order, t.id) > (
                               SELECT sort_order, id FROM launch_template WHERE id = ?
                           )
                         ORDER BY t.sort_order ASC, t.id ASC
                         LIMIT ?",
                    )
                    .bind(pid)
                    .bind(&cursor)
                    .bind(limit + 1)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as::<_, LaunchTemplateView>(
                        "SELECT id, project_id, spec_version, spec_json
                         FROM launch_template
                         WHERE project_id = ?
                         ORDER BY sort_order ASC, id ASC
                         LIMIT ?",
                    )
                    .bind(pid)
                    .bind(limit + 1)
                    .fetch_all(cx.db())
                    .await?
                };

                let has_more = rows.len() as i64 > limit;
                if has_more {
                    rows.truncate(limit as usize);
                }
                let next = if has_more {
                    rows.last().map(|r| r.id.clone())
                } else {
                    None
                };

                Ok(Listing::new(rows, next))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;
    use crate::entities::ProjectId;

    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn list_launch_templates_filters_by_project() {
        let dir = tempfile::TempDir::new().unwrap();
        let (cx, bus) = ctx(&dir).await;

        let other_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO project (id, workspace_id, name, source_kind) VALUES (?, ?, ?, ?)",
        )
        .bind(&other_id)
        .bind("00000000-0000-0000-0000-000000000001")
        .bind("OtherProject")
        .bind("blank")
        .execute(cx.db())
        .await
        .unwrap();

        bus.execute(NewLaunchTemplateCmd {
            id: crate::entities::LaunchTemplateId::mint(),
            project_id: ProjectId::new(UNFILED).as_str().to_owned(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();
        bus.execute(NewLaunchTemplateCmd {
            id: crate::entities::LaunchTemplateId::mint(),
            project_id: other_id.clone(),
            spec_version: 1,
            spec_json: "{}".to_owned(),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.items[0].project_id, UNFILED);
    }

    async fn seed_n(bus: &crate::shared::Bus<Ctx>, n: usize) {
        for _ in 0..n {
            bus.execute(NewLaunchTemplateCmd {
                id: crate::entities::LaunchTemplateId::mint(),
                project_id: ProjectId::new(UNFILED).as_str().to_owned(),
                spec_version: 1,
                spec_json: "{}".to_owned(),
            })
            .await
            .unwrap();
        }
    }

    // -- Scenario: Unbounded listing is explicit -------------------------------

    #[tokio::test]
    async fn list_all_returns_every_row_for_project() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;
        seed_n(&bus, 3).await;

        let listing = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();

        assert_eq!(listing.items.len(), 3);
        assert!(listing.next.is_none());
    }

    // -- Scenario: A bounded offset page returns a continuation cursor ---------

    #[tokio::test]
    async fn offset_pagination_has_next_when_more_remain() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;
        seed_n(&bus, 5).await;

        let page1 = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: Some(3),
                offset: Some(0),
                after: None,
            })
            .await
            .unwrap();

        assert_eq!(page1.items.len(), 3);
        assert!(page1.next.is_some(), "expected a continuation cursor");
    }

    #[tokio::test]
    async fn offset_pagination_last_page_has_no_next() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;
        seed_n(&bus, 3).await;

        // Page size == total: no continuation expected.
        let listing = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: Some(3),
                offset: Some(0),
                after: None,
            })
            .await
            .unwrap();

        assert!(listing.next.is_none());
    }

    // -- Scenario: Cursor pages continue from a last-seen id -------------------

    #[tokio::test]
    async fn cursor_pagination_second_page_completes_the_listing() {
        let dir = tempfile::TempDir::new().unwrap();
        let (_cx, bus) = ctx(&dir).await;
        seed_n(&bus, 5).await;

        // Establish a stable ordering, then take the 3rd id as the cursor.
        let all = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(all.items.len(), 5);
        let cursor = all.items[2].id.clone();
        let page1_ids: std::collections::HashSet<_> =
            all.items[..3].iter().map(|t| t.id.clone()).collect();

        let page2 = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: Some(3),
                offset: None,
                after: Some(cursor),
            })
            .await
            .unwrap();

        // Two rows remain after the cursor; no continuation.
        assert_eq!(page2.items.len(), 2);
        assert!(page2.next.is_none());

        // No overlap with the consumed prefix.
        let page2_ids: std::collections::HashSet<_> =
            page2.items.iter().map(|t| t.id.clone()).collect();
        assert!(page1_ids.is_disjoint(&page2_ids));
    }
}
