//! Client → daemon frame parsing. Internally tagged on `type`. Daemon → client
//! frames are constructed ad hoc with `serde_json` where they are emitted.

use serde::Deserialize;
use std::collections::BTreeMap;

// Single source of truth for the session-event wire version (contracts-rs, R9).
pub const SUPPORTED_VERSIONS: &[u32] = &[contracts::SESSION_EVENT_WIRE_VERSION];

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientFrame {
    Hello {
        versions: Vec<u32>,
        #[serde(default)]
        capabilities: Option<Vec<String>>,
    },
    Spawn(SpawnFrame),
    Kill {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    Stop {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    List,
    Subscribe {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    Unsubscribe {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    Input {
        #[serde(rename = "sessionId")]
        session_id: String,
    },
    Resize {
        #[serde(rename = "sessionId")]
        session_id: String,
        cols: u16,
        rows: u16,
    },
    Ack {
        #[serde(rename = "sessionId")]
        session_id: String,
        bytes: i64,
    },
    Upgrade,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SpawnFrame {
    #[serde(rename = "sessionId")]
    pub session_id: String,
    #[serde(default)]
    pub resume: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Option<BTreeMap<String, String>>,
    pub token: String,
    pub cols: u16,
    pub rows: u16,
    pub cwd: String,
}

pub fn parse_client_frame(meta: &[u8]) -> Option<ClientFrame> {
    serde_json::from_slice(meta).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hello_with_capabilities() {
        let f =
            parse_client_frame(br#"{"type":"hello","versions":[1],"capabilities":["snapshot"]}"#)
                .unwrap();
        match f {
            ClientFrame::Hello {
                versions,
                capabilities,
            } => {
                assert_eq!(versions, vec![1]);
                assert_eq!(capabilities, Some(vec!["snapshot".to_string()]));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn parses_spawn_minimal() {
        let f = parse_client_frame(
            br#"{"type":"spawn","sessionId":"s1","args":[],"token":"t","cols":80,"rows":24,"cwd":"/tmp"}"#,
        )
        .unwrap();
        match f {
            ClientFrame::Spawn(s) => {
                assert_eq!(s.session_id, "s1");
                assert!(s.command.is_none());
                assert_eq!(s.cols, 80);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn parses_ack_and_resize() {
        assert!(matches!(
            parse_client_frame(br#"{"type":"ack","sessionId":"s","bytes":100}"#).unwrap(),
            ClientFrame::Ack { bytes: 100, .. }
        ));
        assert!(matches!(
            parse_client_frame(br#"{"type":"resize","sessionId":"s","cols":120,"rows":40}"#)
                .unwrap(),
            ClientFrame::Resize {
                cols: 120,
                rows: 40,
                ..
            }
        ));
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(parse_client_frame(br#"{"type":"bogus"}"#).is_none());
    }

    #[test]
    fn supported_versions_sourced_from_contracts() {
        assert_eq!(SUPPORTED_VERSIONS, &[contracts::SESSION_EVENT_WIRE_VERSION]);
    }
}
