//! Steam Web API for one's own account. The key is the access token of a signed-in Steam web session
//! (`webapi_token`, the one the Steam website itself uses). Most methods here are the ones the Steam client and
//! website call; Valve doesn't document them officially, so every answer is read defensively.

mod cards;
mod friends;
mod library;
mod profile;
mod security;
mod token;
mod wishlist;

use std::time::Duration;

use serde_json::Value;

pub use cards::{card_drops, has_vac, CardGame};
pub use library::{icon_url, OwnedGame};
pub use profile::{ItemKind, ItemSlot, PlayerSummary, RecentGame, Theme};
pub use security::Session;
pub use token::{now, Token};
pub use wishlist::{capsule_url, WishItem};

const API: &str = "https://api.steampowered.com";
const STORE: &str = "https://store.steampowered.com";

/// Why a request failed. `Display` is for the log (never contains the token).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteamError {
    /// The token was refused or has expired: sign in again.
    Unauthorized,
    /// Too many requests: wait a little.
    RateLimited,
    /// Steam answered `method` with an error: HTTP status and EResult (1 = OK, 15 = access denied...).
    Refused { method: String, status: u16, eresult: Option<u32> },
    Network(String),
    BadAnswer(String),
    /// Not a Steam access token.
    BadToken,
}

impl std::fmt::Display for SteamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteamError::Unauthorized => f.write_str("the access token was refused or has expired"),
            SteamError::RateLimited => f.write_str("too many requests (HTTP 429)"),
            SteamError::Refused { method, status, eresult } => {
                write!(f, "{method}: HTTP {status}, eresult {}", eresult.map_or("-".into(), |e| e.to_string()))
            }
            SteamError::Network(e) => write!(f, "network: {e}"),
            SteamError::BadAnswer(e) => write!(f, "unexpected answer: {e}"),
            SteamError::BadToken => f.write_str("not a Steam access token"),
        }
    }
}

pub type Result<T> = std::result::Result<T, SteamError>;

/// The Web API on behalf of one account.
pub struct Client {
    agent: ureq::Agent,
    token: Token,
    /// Steam's name of the language for names and descriptions: "english", "russian".
    language: String,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("SteamCenter/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Steam's own name of the interface language.
pub fn steam_language(lang: &str) -> &'static str {
    match lang {
        "ru" => "russian",
        _ => "english",
    }
}

/// The answer of a call to `method`: errors by HTTP status and the `x-eresult` header, the body as JSON (empty
/// body - null).
fn answer(method: &str, r: std::result::Result<ureq::Response, ureq::Error>) -> Result<Value> {
    let resp = r.map_err(|e| failed(method, e))?;
    match eresult(&resp) {
        // Access denied / expired / not logged on: the token is no longer good.
        Some(15) | Some(21) | Some(27) => return Err(SteamError::Unauthorized),
        Some(84) => return Err(SteamError::RateLimited),
        Some(e) if e != 1 => return Err(SteamError::Refused { method: method.into(), status: resp.status(), eresult: Some(e) }),
        _ => {}
    }
    let text = resp.into_string().map_err(|e| SteamError::Network(e.to_string()))?;
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| SteamError::BadAnswer(e.to_string()))
}

/// A request that failed outright: by its HTTP status, or the network.
fn failed(method: &str, e: ureq::Error) -> SteamError {
    match e {
        ureq::Error::Status(401 | 403, _) => SteamError::Unauthorized,
        ureq::Error::Status(429, _) => SteamError::RateLimited,
        ureq::Error::Status(status, resp) => SteamError::Refused { method: method.into(), status, eresult: eresult(&resp) },
        e => SteamError::Network(e.to_string()),
    }
}

fn eresult(r: &ureq::Response) -> Option<u32> {
    r.header("x-eresult")?.trim().parse().ok()
}

impl Client {
    pub fn new(token: Token, lang: &str) -> Self {
        Self { agent: agent(), token, language: steam_language(lang).into() }
    }

    pub fn steamid(&self) -> &str {
        &self.token.steamid
    }

    /// `method` - "Interface/Method/v1". Returns the `response` object of the answer.
    fn get(&self, method: &str, params: &[(&str, &str)]) -> Result<Value> {
        let mut req = self.agent.get(&format!("{API}/{method}/")).query("access_token", &self.token.value);
        for (k, v) in params {
            req = req.query(k, v);
        }
        Ok(answer(method, req.call())?.get_mut("response").map(Value::take).unwrap_or(Value::Null))
    }

    fn post(&self, method: &str, params: &[(String, String)]) -> Result<Value> {
        let mut form: Vec<(&str, &str)> = vec![("access_token", &self.token.value)];
        form.extend(params.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        Ok(answer(method, self.agent.post(&format!("{API}/{method}/")).send_form(&form))?.get_mut("response").map(Value::take).unwrap_or(Value::Null))
    }
}

/// The `webapi_token` of a signed-in web session: the store hands it to its own pages. `login_secure` is the value
/// of the `steamLoginSecure` cookie (`<steamid>%7C%7C<jwt>`).
pub fn web_token(login_secure: &str) -> Result<Token> {
    let r = agent()
        .get(&format!("{STORE}/pointssummary/ajaxgetasyncconfig"))
        .set("Cookie", &format!("steamLoginSecure={login_secure}"))
        .call();
    let v = answer("store/ajaxgetasyncconfig", r)?;
    match v.pointer("/data/webapi_token").and_then(Value::as_str) {
        Some(t) if !t.is_empty() => Token::parse(t),
        _ => Err(SteamError::Unauthorized),
    }
}

/// A number Steam sends either as a JSON number or as a string (64-bit ids), as text.
fn text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

fn num(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_u64().unwrap_or(0),
        Value::String(s) => s.parse().unwrap_or(0),
        _ => 0,
    }
}

fn list<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_come_as_numbers_or_strings() {
        let v: Value = serde_json::from_str(r#"{"a":"76561198000000000","b":42,"c":null}"#).unwrap();
        assert_eq!(text(&v["a"]), "76561198000000000");
        assert_eq!(text(&v["b"]), "42");
        assert_eq!(text(&v["c"]), "");
        assert_eq!(num(&v["a"]), 76561198000000000);
        assert_eq!(num(&v["c"]), 0);
        assert!(list(&v, "missing").is_empty());
    }

    #[test]
    fn steam_language_names() {
        assert_eq!(steam_language("ru"), "russian");
        assert_eq!(steam_language("en"), "english");
    }
}
