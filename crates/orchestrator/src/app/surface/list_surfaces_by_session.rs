use serde::Deserialize;

use crate::app::surface::SurfaceView;
use crate::context::Ctx;
use crate::shared::errors::Result;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};

/// A session's surfaces, live-first.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
/// The cursor is the `created_at` of the last row on the previous page; live
/// surfaces sort first, so cursor paging across a mixed-status set may skip some
/// non-live rows (callers needing strict stability use offset paging or all).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSurfacesBySession {
    pub session: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListSurfacesBySession {
    type Out = Listing<SurfaceView>;
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
                // Live-first, then by insertion order. Ordering applied by app; infra repo
                // returns stable id order only.
                let items = sqlx::query_as::<_, SurfaceView>(
                    "SELECT id, session_id, kind, cwd, status, placement, spawned_at
                     FROM surface
                     WHERE session_id = ?
                     ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC",
                )
                .bind(&self.session)
                .fetch_all(cx.db())
                .await?;
                Ok(Listing::new(items, None))
            }

            Page::Offset { limit, offset } => {
                let fetch = (limit as i64) + 1;
                let rows = sqlx::query_as::<_, SurfaceView>(
                    "SELECT id, session_id, kind, cwd, status, placement, spawned_at
                     FROM surface
                     WHERE session_id = ?
                     ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC
                     LIMIT ? OFFSET ?",
                )
                .bind(&self.session)
                .bind(fetch)
                .bind(offset as i64)
                .fetch_all(cx.db())
                .await?;

                let has_next = rows.len() as u32 > limit;
                let next = has_next.then(|| (offset + limit).to_string());
                let items: Vec<SurfaceView> = rows.into_iter().take(limit as usize).collect();
                Ok(Listing::new(items, next))
            }

            Page::Cursor { after, limit } => {
                let fetch = (limit as i64) + 1;
                let rows: Vec<CursorRow> = if let Some(cursor) = after {
                    sqlx::query_as::<_, CursorRow>(
                        "SELECT id, session_id, kind, cwd, status, placement, spawned_at, created_at
                         FROM surface
                         WHERE session_id = ? AND created_at > ?
                         ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC
                         LIMIT ?",
                    )
                    .bind(&self.session)
                    .bind(cursor)
                    .bind(fetch)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as::<_, CursorRow>(
                        "SELECT id, session_id, kind, cwd, status, placement, spawned_at, created_at
                         FROM surface
                         WHERE session_id = ?
                         ORDER BY CASE status WHEN 'live' THEN 0 ELSE 1 END ASC, created_at ASC
                         LIMIT ?",
                    )
                    .bind(&self.session)
                    .bind(fetch)
                    .fetch_all(cx.db())
                    .await?
                };

                let has_next = rows.len() as u32 > limit;
                let next_cursor = has_next
                    .then(|| rows.get(limit as usize - 1).map(|r| r.created_at.clone()))
                    .flatten();
                let items: Vec<SurfaceView> = rows
                    .into_iter()
                    .take(limit as usize)
                    .map(|r| r.view)
                    .collect();
                Ok(Listing::new(items, next_cursor))
            }
        }
    }
}

/// A `SurfaceView` plus the `created_at` cursor-key column needed to mint the
/// continuation cursor without re-querying.
#[derive(sqlx::FromRow)]
struct CursorRow {
    #[sqlx(flatten)]
    view: SurfaceView,
    created_at: String,
}
