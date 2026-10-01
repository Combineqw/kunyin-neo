//! Host-neutral helpers for the LX Music remote-sync session boundary.
//!
//! The network client remains owned by each host (Electron currently owns the
//! HTTP/WebSocket and crypto stack).  These helpers keep the small, pure input
//! contract shared with the Tauri shell and the N-API adapter.

use serde::{Deserialize, Serialize};

use crate::error::{AuroraError, Result};

/// The subset of a persisted sync session that is safe to pass between hosts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncSession {
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "aesKey")]
    pub aes_key: String,
    #[serde(rename = "serverName", default)]
    pub server_name: String,
}

/// Normalize the server URL before the protocol appends `/hello`, `/ah`, or
/// `/socket`.  This mirrors the existing Electron client and accepts `ws://`
/// input for settings that are shared with WebSocket-oriented clients.
pub fn normalize_base_url(input: &str) -> Result<String> {
    let value = input.trim();
    if value.is_empty() {
        return Err(AuroraError::InvalidInput("同步服务器地址为空".into()));
    }

    let lower = value.to_ascii_lowercase();
    let normalized = if lower.starts_with("http://") || lower.starts_with("https://") {
        value.to_string()
    } else if lower.starts_with("ws://") {
        format!("http://{}", &value["ws://".len()..])
    } else if lower.starts_with("wss://") {
        format!("https://{}", &value["wss://".len()..])
    } else {
        format!("http://{value}")
    };

    Ok(normalized.trim_end_matches('/').to_string())
}

/// Parse and validate the persisted session JSON.
///
/// Invalid JSON or missing required fields is treated as an absent session by
/// the host adapters.  The result is canonicalized to the three fields above,
/// so future persisted fields cannot accidentally become part of the contract.
pub fn parse_session_json(input: &str) -> Result<Option<SyncSession>> {
    let value: serde_json::Value = serde_json::from_str(input)?;
    let Some(object) = value.as_object() else {
        return Ok(None);
    };
    let Some(client_id) = object.get("clientId").and_then(serde_json::Value::as_str) else {
        return Ok(None);
    };
    let Some(aes_key) = object.get("aesKey").and_then(serde_json::Value::as_str) else {
        return Ok(None);
    };
    if client_id.is_empty() || aes_key.is_empty() {
        return Ok(None);
    }
    let server_name = object
        .get("serverName")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    Ok(Some(SyncSession {
        client_id: client_id.to_string(),
        aes_key: aes_key.to_string(),
        server_name: server_name.to_string(),
    }))
}

/// Serialize a validated session using the camelCase JSON used by the hosts.
pub fn session_json(input: &str) -> Result<Option<String>> {
    parse_session_json(input).and_then(|session| {
        session
            .map(|value| serde_json::to_string(&value).map_err(AuroraError::from))
            .transpose()
    })
}

#[cfg(test)]
mod tests {
    use super::{normalize_base_url, parse_session_json, session_json};

    #[test]
    fn normalizes_http_and_websocket_inputs() {
        assert_eq!(
            normalize_base_url("example.test/").unwrap(),
            "http://example.test"
        );
        assert_eq!(
            normalize_base_url("ws://example.test///").unwrap(),
            "http://example.test"
        );
        assert_eq!(
            normalize_base_url("WSS://example.test/").unwrap(),
            "https://example.test"
        );
        assert_eq!(
            normalize_base_url(" https://example.test/api/ ").unwrap(),
            "https://example.test/api"
        );
    }

    #[test]
    fn rejects_empty_server_url() {
        assert!(normalize_base_url("  ").is_err());
    }

    #[test]
    fn validates_and_canonicalizes_persisted_session() {
        let session = parse_session_json(r#"{"clientId":"id","aesKey":"key","extra":1}"#)
            .unwrap()
            .unwrap();
        assert_eq!(session.client_id, "id");
        assert_eq!(session.server_name, "");
        assert_eq!(
            session_json(r#"{"clientId":"id","aesKey":"key","serverName":"LX"}"#)
                .unwrap()
                .unwrap(),
            r#"{"clientId":"id","aesKey":"key","serverName":"LX"}"#
        );
    }

    #[test]
    fn missing_session_fields_are_absent() {
        assert!(parse_session_json(r#"{"clientId":"id"}"#)
            .unwrap()
            .is_none());
        assert!(parse_session_json("[]").unwrap().is_none());
    }
}
