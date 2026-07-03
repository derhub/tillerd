use serde::Deserialize;

use crate::app::workspace::WorkspaceView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// List all workspaces, pinned-first then by sort order.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
///
/// Cursor format: opaque decimal string encoding the next-page offset (matches
/// the prior repo implementation byte-for-byte).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListWorkspaces {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListWorkspaces {
    type Out = Listing<WorkspaceView>;

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
                let items = sqlx::query_as::<_, WorkspaceView>(
                    "SELECT id, name
                     FROM workspace
                     ORDER BY pinned DESC, sort_order",
                )
                .fetch_all(cx.db())
                .await?;
                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                // Fetch limit+1 to detect whether a next page exists.
                let items = sqlx::query_as::<_, WorkspaceView>(
                    "SELECT id, name
                     FROM workspace
                     ORDER BY pinned DESC, sort_order
                     LIMIT ? OFFSET ?",
                )
                .bind(limit as i64 + 1)
                .bind(offset as i64)
                .fetch_all(cx.db())
                .await?;
                let has_more = items.len() > limit as usize;
                let items: Vec<WorkspaceView> = items.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    Some((offset + limit).to_string())
                } else {
                    None
                };
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                // Cursor is the next-page offset encoded as a decimal string.
                let offset: i64 = match after.as_deref() {
                    None => 0,
                    Some(s) => s.parse::<i64>().unwrap_or(0),
                };

                // Fetch limit+1 to detect whether a next page exists.
                let items = sqlx::query_as::<_, WorkspaceView>(
                    "SELECT id, name
                     FROM workspace
                     ORDER BY pinned DESC, sort_order
                     LIMIT ? OFFSET ?",
                )
                .bind(limit as i64 + 1)
                .bind(offset)
                .fetch_all(cx.db())
                .await?;

                let has_more = items.len() > limit as usize;
                let items: Vec<WorkspaceView> = items.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    Some((offset + limit as i64).to_string())
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
    use crate::app::workspace::test_util::*;
    use crate::app::workspace::PinWorkspace;
    use crate::shared::message::Command;

    // Scenario: A query reads and does not mutate -- ListWorkspaces.
    #[tokio::test]
    async fn list_workspaces_returns_all_workspaces() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-list-a", "A").await;
        insert_workspace(&cx, "ws-list-b", "B").await;
        let listing = ListWorkspaces {
            limit: None,
            offset: None,
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        let ids: Vec<&str> = listing.items.iter().map(|w| w.id.as_str()).collect();
        assert!(ids.contains(&"ws-list-a"));
        assert!(ids.contains(&"ws-list-b"));
    }

    // Scenario: A pinned item sorts ahead of unpinned.
    #[tokio::test]
    async fn list_returns_pinned_workspaces_first() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-pin-unpinned", "Unpinned").await;
        insert_workspace(&cx, "ws-pin-pinned", "Pinned").await;
        PinWorkspace {
            id: "ws-pin-pinned".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        let listing = ListWorkspaces {
            limit: None,
            offset: None,
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        let first = listing.items.first().expect("at least one item");
        assert_eq!(
            first.id, "ws-pin-pinned",
            "pinned workspace must appear first"
        );
    }

    // Scenario: A bounded page returns a continuation cursor.
    #[tokio::test]
    async fn cursor_page_returns_next_when_more_rows_remain() {
        let cx = ctx().await;
        // Default workspace is already seeded; add one more.
        insert_workspace(&cx, "ws-cur-1", "Extra").await;

        let listing = ListWorkspaces {
            limit: Some(1),
            offset: None,
            after: Some("0".to_owned()),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert_eq!(listing.items.len(), 1);
        assert!(
            listing.next.is_some(),
            "next cursor must be set when more rows remain"
        );
    }

    #[tokio::test]
    async fn cursor_page_at_end_has_no_next() {
        let cx = ctx().await;
        // Only the seeded Default workspace. One result, no next.
        let listing = ListWorkspaces {
            limit: Some(10),
            offset: None,
            after: Some("0".to_owned()),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert!(!listing.items.is_empty());
        assert!(
            listing.next.is_none(),
            "no next when all rows fit in the page"
        );
    }

    #[tokio::test]
    async fn cursor_continues_from_returned_cursor() {
        let cx = ctx().await;
        // Default workspace seeded. Add two more for 3 total.
        insert_workspace(&cx, "ws-seq-2", "W2").await;
        insert_workspace(&cx, "ws-seq-3", "W3").await;

        let page1 = ListWorkspaces {
            limit: Some(1),
            offset: None,
            after: Some("0".to_owned()),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert_eq!(page1.items.len(), 1);
        let cursor = page1.next.expect("must have next after page 1 of 3");

        let page2 = ListWorkspaces {
            limit: Some(1),
            offset: None,
            after: Some(cursor),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert_eq!(page2.items.len(), 1);
        // page2 item must differ from page1 item.
        assert_ne!(
            page2.items[0].id, page1.items[0].id,
            "page 2 must advance past page 1"
        );
    }

    // Scenario: offset pagination.
    #[tokio::test]
    async fn offset_page_returns_correct_slice() {
        let cx = ctx().await;
        insert_workspace(&cx, "ws-off-2", "W2").await;
        insert_workspace(&cx, "ws-off-3", "W3").await;

        let page = ListWorkspaces {
            limit: Some(2),
            offset: Some(0),
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        assert_eq!(page.items.len(), 2);
        // Next cursor is present because there are 3 rows and only 2 were taken.
        assert!(page.next.is_some());

        let page2 = ListWorkspaces {
            limit: Some(2),
            offset: Some(2),
            after: None,
        }
        .handle(&cx)
        .await
        .unwrap();
        assert_eq!(page2.items.len(), 1);
        assert!(page2.next.is_none());
    }
}
