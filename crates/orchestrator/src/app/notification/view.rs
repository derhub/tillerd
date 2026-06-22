use serde::Serialize;

/// Flat read model for a notification row. Serializes to the SDK `NotificationEvent`
/// wire shape -- the same camelCase JSON the host `NotificationWire` produced from a
/// `NotificationRecord` (internal columns `actions_json`/`read`/`snooze_until` are
/// not on the wire and so are omitted here).
///
/// The `Option` fields carry `skip_serializing_if` so absent values drop out of the
/// JSON exactly as the host struct did.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NotificationView {
    pub id: String,
    pub category: String,
    pub severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub ts: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface_id: Option<String>,
}
