//! The access token: a JWT whose `sub` is the SteamID64 and `exp` the end of its life.

use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::Value;

use crate::{Result, SteamError};

#[derive(Clone, PartialEq, Eq)]
pub struct Token {
    pub value: String,
    pub steamid: String,
    /// Unix time, seconds.
    pub expires: u64,
}

/// The token itself never gets into logs.
impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Token({}, expires {})", self.steamid, self.expires)
    }
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl Token {
    /// Accepts the token itself, the `steamLoginSecure` cookie (`<steamid>%7C%7C<jwt>`) or the whole
    /// `ajaxgetasyncconfig` answer someone copied from the browser.
    pub fn parse(raw: &str) -> Result<Self> {
        let value = normalize(raw);
        let payload = value.split('.').nth(1).ok_or(SteamError::BadToken)?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload.trim_end_matches('='))
            .map_err(|_| SteamError::BadToken)?;
        let claims: Value = serde_json::from_slice(&bytes).map_err(|_| SteamError::BadToken)?;
        let steamid = claims.get("sub").and_then(Value::as_str).filter(|s| s.len() == 17 && s.bytes().all(|b| b.is_ascii_digit()));
        let steamid = steamid.ok_or(SteamError::BadToken)?.to_string();
        let expires = claims.get("exp").and_then(Value::as_u64).unwrap_or(0);
        Ok(Self { value, steamid, expires })
    }

    /// Expired, or expires within `secs` seconds.
    pub fn expires_within(&self, secs: u64) -> bool {
        self.expires <= now() + secs
    }
}

fn normalize(raw: &str) -> String {
    let raw = raw.trim().trim_matches('"');
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        if let Some(t) = v.pointer("/data/webapi_token").and_then(Value::as_str) {
            return t.to_string();
        }
    }
    match raw.split_once("%7C%7C").or_else(|| raw.split_once("||")) {
        Some((_, t)) => t.to_string(),
        None => raw.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jwt(claims: &str) -> String {
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!("{}.{}.sig", e.encode(r#"{"typ":"JWT"}"#), e.encode(claims))
    }

    #[test]
    fn reads_steamid_and_expiry() {
        let t = Token::parse(&jwt(r#"{"sub":"76561198000000001","exp":4000000000}"#)).unwrap();
        assert_eq!(t.steamid, "76561198000000001");
        assert_eq!(t.expires, 4000000000);
        assert!(!t.expires_within(3600));
        let old = Token::parse(&jwt(r#"{"sub":"76561198000000001","exp":5}"#)).unwrap();
        assert!(old.expires_within(0));
    }

    #[test]
    fn accepts_cookie_and_json_forms() {
        let j = jwt(r#"{"sub":"76561198000000001","exp":4000000000}"#);
        assert_eq!(Token::parse(&format!("76561198000000001%7C%7C{j}")).unwrap().value, j);
        assert_eq!(Token::parse(&format!("  \"{j}\" ")).unwrap().value, j);
        let page = format!(r#"{{"success":1,"data":{{"webapi_token":"{j}"}}}}"#);
        assert_eq!(Token::parse(&page).unwrap().value, j);
    }

    #[test]
    fn rejects_what_is_not_a_token() {
        assert_eq!(Token::parse("hello"), Err(SteamError::BadToken));
        assert_eq!(Token::parse(&jwt(r#"{"sub":"bob"}"#)), Err(SteamError::BadToken));
        assert!(!format!("{:?}", Token::parse(&jwt(r#"{"sub":"76561198000000001"}"#)).unwrap()).contains("sig"));
    }
}
