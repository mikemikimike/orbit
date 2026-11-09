use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Opaque keyset cursor: `(created_at, id)` base64url-encoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub created_at: DateTime<Utc>,
    pub id: Uuid,
}

impl Cursor {
    pub fn new(created_at: DateTime<Utc>, id: Uuid) -> Self {
        Self { created_at, id }
    }

    pub fn encode(&self) -> String {
        let raw = format!("{}|{}", self.created_at.timestamp_micros(), self.id);
        URL_SAFE_NO_PAD.encode(raw)
    }

    pub fn decode(value: &str) -> Option<Self> {
        let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
        let raw = String::from_utf8(decoded).ok()?;
        let (ts, id) = raw.split_once('|')?;
        let micros: i64 = ts.parse().ok()?;
        let created_at = DateTime::<Utc>::from_timestamp_micros(micros)?;
        Some(Self {
            created_at,
            id: Uuid::parse_str(id).ok()?,
        })
    }
}

pub const DEFAULT_PAGE_LIMIT: u32 = 25;
pub const MAX_PAGE_LIMIT: u32 = 100;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(default)]
    pub total: Option<i64>,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, next_cursor: Option<String>) -> Self {
        Self {
            items,
            next_cursor,
            total: None,
        }
    }
    pub fn empty() -> Self {
        Self {
            items: Vec::new(),
            next_cursor: None,
            total: None,
        }
    }
}

/// Clamp a user-supplied limit into the supported range.
pub fn clamp_limit(limit: Option<u32>) -> i64 {
    limit.unwrap_or(DEFAULT_PAGE_LIMIT).clamp(1, MAX_PAGE_LIMIT) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_roundtrip() {
        let cursor = Cursor::new(Utc::now(), Uuid::now_v7());
        let encoded = cursor.encode();
        let decoded = Cursor::decode(&encoded).expect("decodable");
        assert_eq!(decoded.id, cursor.id);
        assert_eq!(decoded.created_at, cursor.created_at);
        assert!(Cursor::decode("not-base64!").is_none());
    }

    #[test]
    fn limits_are_clamped() {
        assert_eq!(clamp_limit(None), DEFAULT_PAGE_LIMIT as i64);
        assert_eq!(clamp_limit(Some(0)), 1);
        assert_eq!(clamp_limit(Some(1000)), MAX_PAGE_LIMIT as i64);
    }
}
