use serde::Deserialize;
use sqlx::AssertSqlSafe;

use crate::app::command::CommandView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::pagination::{Listing, Page};
use crate::shared::Result;

/// Projection columns for the `CommandView` read model.
const SELECT: &str = "SELECT id, name, origin, cli, args_json, env_json
                      FROM command
                      WHERE deleted_at IS NULL";

/// List all library commands, optionally filtered by origin.
///
/// Pagination mode is rebuilt from the primitive wire inputs in `handle`:
/// `after` present -> cursor page; else `limit` present -> offset page; else all.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommands {
    /// `"prebuilt"` or `"custom"` to restrict; `None` lists every origin.
    pub origin: Option<String>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub after: Option<String>,
}

impl Query<Ctx> for ListCommands {
    type Out = Listing<CommandView>;

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

        let origin_filter = self.origin.as_deref();

        match page {
            Page::All => {
                let items: Vec<CommandView> = if let Some(origin) = origin_filter {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{SELECT} AND origin = ? ORDER BY pinned DESC, sort_order ASC"
                    )))
                    .bind(origin)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{SELECT} ORDER BY pinned DESC, sort_order ASC"
                    )))
                    .fetch_all(cx.db())
                    .await?
                };
                Ok(Listing::new(items, None))
            }
            Page::Offset { limit, offset } => {
                let items: Vec<CommandView> = if let Some(origin) = origin_filter {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{SELECT} AND origin = ? ORDER BY pinned DESC, sort_order ASC LIMIT ? OFFSET ?"
                    )))
                    .bind(origin)
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(cx.db())
                    .await?
                } else {
                    sqlx::query_as(AssertSqlSafe(format!(
                        "{SELECT} ORDER BY pinned DESC, sort_order ASC LIMIT ? OFFSET ?"
                    )))
                    .bind(limit as i64)
                    .bind(offset as i64)
                    .fetch_all(cx.db())
                    .await?
                };
                Ok(Listing::new(items, None))
            }
            Page::Cursor { after, limit } => {
                // Fetch limit+1 to detect whether a next page exists without a
                // COUNT query. If we get more than limit rows back, a next page
                // exists; we truncate to limit before returning.
                let fetch_n = limit as i64 + 1;
                let rows: Vec<CommandView> = if let Some(cursor) = after {
                    if let Some(origin) = origin_filter {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "WITH anchor AS (
                                 SELECT pinned, sort_order FROM command WHERE id = ?
                             )
                             {SELECT} AND origin = ?
                               AND (pinned < (SELECT pinned FROM anchor)
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order > (SELECT sort_order FROM anchor))
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order = (SELECT sort_order FROM anchor)
                                        AND id > ?))
                             ORDER BY pinned DESC, sort_order ASC, id ASC
                             LIMIT ?"
                        )))
                        .bind(&cursor)
                        .bind(origin)
                        .bind(&cursor)
                        .bind(fetch_n)
                        .fetch_all(cx.db())
                        .await?
                    } else {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "WITH anchor AS (
                                 SELECT pinned, sort_order FROM command WHERE id = ?
                             )
                             {SELECT}
                               AND (pinned < (SELECT pinned FROM anchor)
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order > (SELECT sort_order FROM anchor))
                                    OR (pinned = (SELECT pinned FROM anchor)
                                        AND sort_order = (SELECT sort_order FROM anchor)
                                        AND id > ?))
                             ORDER BY pinned DESC, sort_order ASC, id ASC
                             LIMIT ?"
                        )))
                        .bind(&cursor)
                        .bind(&cursor)
                        .bind(fetch_n)
                        .fetch_all(cx.db())
                        .await?
                    }
                } else {
                    // first page
                    if let Some(origin) = origin_filter {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "{SELECT} AND origin = ?
                             ORDER BY pinned DESC, sort_order ASC, id ASC
                             LIMIT ?"
                        )))
                        .bind(origin)
                        .bind(fetch_n)
                        .fetch_all(cx.db())
                        .await?
                    } else {
                        sqlx::query_as(AssertSqlSafe(format!(
                            "{SELECT} ORDER BY pinned DESC, sort_order ASC, id ASC LIMIT ?"
                        )))
                        .bind(fetch_n)
                        .fetch_all(cx.db())
                        .await?
                    }
                };
                let has_more = rows.len() > limit as usize;
                let items: Vec<CommandView> = rows.into_iter().take(limit as usize).collect();
                let next = if has_more {
                    items.last().map(|c| c.id.clone())
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
    use crate::app::command::new_command::NewCommand;
    use crate::app::command::test_util::*;
    use crate::shared::Bus;

    #[tokio::test]
    async fn list_commands_returns_all_including_seeded_prebuilt() {
        let bus = Bus::new(ctx().await);
        let listing = bus
            .query(ListCommands {
                origin: None,
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        // The migration seeds login-shell as prebuilt.
        assert!(listing.items.iter().any(|c| c.name == "login-shell"));
    }

    #[tokio::test]
    async fn list_commands_by_origin_returns_only_matching_origin() {
        let bus = Bus::new(ctx().await);
        bus.execute(NewCommand {
            id: crate::entities::command::CommandId::mint(),
            name: "custom-one".to_owned(),
            cli: "/bin/bash".to_owned(),
            args: vec![],
            env: std::collections::HashMap::new(),
        })
        .await
        .unwrap();

        let custom_only = bus
            .query(ListCommands {
                origin: Some("custom".to_owned()),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(custom_only.items.iter().all(|c| c.origin == "custom"));

        let prebuilt_only = bus
            .query(ListCommands {
                origin: Some("prebuilt".to_owned()),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert!(prebuilt_only.items.iter().all(|c| c.origin == "prebuilt"));
    }
}
