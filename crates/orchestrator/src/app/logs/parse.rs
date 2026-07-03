use serde_json::{Map, Value};

use crate::app::logs::view::LogRecordView;

const RESOURCE_KEYS: &[&str] = &[
    "service.name",
    "service.version",
    "service.instance.id",
    "host.name",
    "process.pid",
];

/// Parse one tracing-subscriber JSON line. `None` on blank or malformed input.
/// `body` is `fields.message`; non-message/name fields and span fields land in
/// `resource` (when the key is a resource key) else `attributes`.
pub fn parse_record(line: &str) -> Option<LogRecordView> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let record = match serde_json::from_str::<Value>(trimmed) {
        Ok(Value::Object(map)) => map,
        _ => return None,
    };

    let fields = as_object(record.get("fields"));
    let body = fields
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let mut attributes = Map::new();
    let mut resource = Map::new();
    let mut place = |key: &str, value: &Value| {
        if key == "message" || key == "name" {
            return;
        }
        if RESOURCE_KEYS.contains(&key) {
            resource.insert(key.to_owned(), value.clone());
        } else {
            attributes.insert(key.to_owned(), value.clone());
        }
    };

    for (key, value) in &fields {
        place(key, value);
    }
    for span in span_list(&record) {
        for (key, value) in &span {
            place(key, value);
        }
    }

    Some(LogRecordView {
        timestamp: str_field(&record, "timestamp"),
        level: str_field(&record, "level"),
        body,
        attributes: Value::Object(attributes),
        resource: Value::Object(resource),
        raw: trimmed.to_owned(),
    })
}

fn str_field(record: &Map<String, Value>, key: &str) -> String {
    record
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn as_object(value: Option<&Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(map)) => map.clone(),
        _ => Map::new(),
    }
}

fn span_list(record: &Map<String, Value>) -> Vec<Map<String, Value>> {
    if let Some(Value::Array(items)) = record.get("spans") {
        return items
            .iter()
            .filter_map(|s| match s {
                Value::Object(map) => Some(map.clone()),
                _ => None,
            })
            .collect();
    }
    if let Some(Value::Object(map)) = record.get("span") {
        return vec![map.clone()];
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn line() -> String {
        json!({
            "timestamp": "2026-06-13T10:00:00.000000Z",
            "level": "INFO",
            "fields": { "message": "spawning pty", "pty.pid": 42 },
            "target": "tillerd::server",
            "spans": [
                { "service.name": "tillerd-daemon", "service.version": "0.0.0", "name": "service" },
                { "session.id": "s1", "component": "daemon", "name": "handle" },
            ],
        })
        .to_string()
    }

    #[test]
    fn maps_timestamp_level_and_body_from_the_tracing_json_shape() {
        let r = parse_record(&line()).unwrap();
        assert_eq!(r.timestamp, "2026-06-13T10:00:00.000000Z");
        assert_eq!(r.level, "INFO");
        assert_eq!(r.body, "spawning pty");
    }

    #[test]
    fn splits_resource_fields_from_attributes() {
        let r = parse_record(&line()).unwrap();
        assert_eq!(
            r.resource,
            json!({ "service.name": "tillerd-daemon", "service.version": "0.0.0" })
        );
        assert_eq!(
            r.attributes,
            json!({ "pty.pid": 42, "session.id": "s1", "component": "daemon" })
        );
    }

    #[test]
    fn returns_none_for_a_blank_line() {
        assert!(parse_record("").is_none());
        assert!(parse_record("   ").is_none());
    }

    #[test]
    fn returns_none_for_a_malformed_line() {
        assert!(parse_record("{not json").is_none());
    }

    #[test]
    fn returns_none_for_a_non_object_json_line() {
        assert!(parse_record("42").is_none());
    }
}
