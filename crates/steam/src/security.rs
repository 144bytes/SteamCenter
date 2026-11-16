//! Security: the devices and browsers signed in to the account (its refresh tokens), with where and when each was
//! used. Revoking one needs a signature only the Steam client and site can make, so it is done on Steam's page.

use serde::Serialize;
use serde_json::Value;

use crate::{list, num, text, Client, Result};

#[derive(Debug, Clone, Serialize)]
pub struct Session {
    pub id: String,
    /// What Steam calls it: a device name, a browser.
    pub description: String,
    /// "client", "browser", "mobile", "unknown".
    pub platform: &'static str,
    pub logged_in: bool,
    /// The session this program uses (the browser of its sign-in window).
    pub current: bool,
    pub first_seen: Option<Seen>,
    pub last_seen: Option<Seen>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Seen {
    /// Unix time.
    pub time: u64,
    pub ip: String,
    /// "City, region, country" as far as Steam knows.
    pub place: String,
}

fn seen(v: Option<&Value>) -> Option<Seen> {
    let v = v?;
    let time = v.get("time").map_or(0, num);
    let place: Vec<String> = ["city", "state", "country"].iter().filter_map(|k| v.get(*k).map(text)).filter(|s| !s.is_empty()).collect();
    let ip = v.get("ip").map(ip_text).unwrap_or_default();
    (time > 0 || !ip.is_empty() || !place.is_empty()).then(|| Seen { time, ip, place: place.join(", ") })
}

/// CMsgIPAddress: `v4` as a number (a.b.c.d with `a` in the high byte) or `v6` as base64 bytes.
fn ip_text(v: &Value) -> String {
    if let Some(n) = v.get("v4").map(num).filter(|&n| n != 0) {
        let b = (n as u32).to_be_bytes();
        return format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3]);
    }
    use base64::Engine;
    let bytes = v.get("v6").and_then(Value::as_str).and_then(|s| base64::engine::general_purpose::STANDARD.decode(s).ok());
    match bytes.and_then(|b| <[u8; 16]>::try_from(b).ok()) {
        Some(b) => std::net::Ipv6Addr::from(b).to_string(),
        None => String::new(),
    }
}

fn sessions(r: &Value) -> Vec<Session> {
    let current = r.get("requesting_token").map(text).unwrap_or_default();
    let mut list: Vec<Session> = list(r, "refresh_tokens")
        .iter()
        .filter_map(|t| {
            let id = t.get("token_id").map(text).filter(|id| !id.is_empty())?;
            Some(Session {
                current: !current.is_empty() && id == current,
                description: t.get("token_description").map(text).unwrap_or_default(),
                // EAuthTokenPlatformType.
                platform: match t.get("platform_type").map_or(0, num) {
                    1 => "client",
                    2 => "browser",
                    3 => "mobile",
                    _ => "unknown",
                },
                logged_in: t.get("logged_in").and_then(Value::as_bool).unwrap_or(false),
                first_seen: seen(t.get("first_seen")),
                last_seen: seen(t.get("last_seen")),
                id,
            })
        })
        .collect();
    // The latest used first.
    list.sort_by_key(|s| std::cmp::Reverse(s.last_seen.as_ref().map_or(0, |l| l.time)));
    list
}

impl Client {
    pub fn sessions(&self) -> Result<Vec<Session>> {
        let r = self.post("IAuthenticationService/EnumerateTokens/v1", &[("include_revoked".into(), "false".into())])?;
        Ok(sessions(&r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessions_are_read() {
        let v: Value = serde_json::from_str(
            r#"{"refresh_tokens":[
                {"token_id":"111","token_description":"DESKTOP-1","platform_type":1,"logged_in":true,
                 "first_seen":{"time":1600000000,"ip":{"v4":3232235777},"city":"Moscow","country":"RU"},
                 "last_seen":{"time":1700000000,"ip":{"v4":3232235778},"country":"RU"}},
                {"token_id":"222","token_description":"Chrome","platform_type":2,
                 "last_seen":{"time":1710000000,"ip":{"v6":"IAENuAAAAAAAAAAAAAAAAQ=="}}},
                {"token_description":"no id"}],
              "requesting_token":"222"}"#,
        )
        .unwrap();
        let s = sessions(&v);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].id.as_str(), s[0].platform, s[0].current), ("222", "browser", true));
        assert_eq!(s[0].last_seen.as_ref().unwrap().ip, "2001:db8::1");
        assert_eq!((s[1].platform, s[1].logged_in, s[1].current), ("client", true, false));
        let first = s[1].first_seen.as_ref().unwrap();
        assert_eq!((first.ip.as_str(), first.place.as_str()), ("192.168.1.1", "Moscow, RU"));
        assert!(s[0].first_seen.is_none());
    }
}
