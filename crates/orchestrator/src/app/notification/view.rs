use serde::Serialize;

/// Flat read model for a notification row. Serializes to the SDK `NotificationView`
/// wire shape -- the same camelCase JSON the host `NotificationWire` produced from a
/// `NotificationRecord` (internal columns `actions_json`/`read`/`snooze_until` are
/// not on the wire and so are omitted here).
///
/// Optionals serialize as null (not omitted) so serialize/deserialize stay symmetric and specta
/// emits a single type rather than _Serialize/_Deserialize variants.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct NotificationView {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: Option<String>,
    pub message: String,
    pub detail: Option<String>,
    pub ts: i64,
    pub session_id: Option<String>,
    pub surface_id: Option<String>,
}
