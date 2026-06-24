use serde::Serialize;
use serde_json::Value;

/// One parsed structured-log line.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LogRecordView {
    pub timestamp: String,
    pub level: String,
    pub body: String,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Unknown))]
    pub attributes: Value,
    #[cfg_attr(feature = "specta", specta(type = specta_typescript::Unknown))]
    pub resource: Value,
    pub raw: String,
}

/// One structured `.log` file under the runtime logs directory.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LogFileView {
    pub name: String,
    pub path: String,
    pub size: u64,
}

/// A bounded window of parsed records plus the byte offsets that produced it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub struct LogTailView {
    pub records: Vec<LogRecordView>,
    pub start: u64,
    pub end: u64,
}
