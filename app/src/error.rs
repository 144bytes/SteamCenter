//! Messages for the window: errors and notices travel as a code from the i18n dictionary plus parameters,
//! the window translates them.

use serde::Serialize;
use serde_json::{Map, Value};
use steamcenter_steam::SteamError;

/// A message for the window: a code from the i18n dictionary plus parameters.
#[derive(Debug, Clone, Serialize)]
pub struct Msg {
    pub code: String,
    pub params: Map<String, Value>,
}

pub fn msg(code: &str) -> Msg {
    Msg { code: code.into(), params: Map::new() }
}

pub fn msg_param(code: &str, name: &str, value: impl ToString) -> Msg {
    let mut m = msg(code);
    m.params.insert(name.into(), Value::String(value.to_string()));
    m
}

pub fn msg_detail(code: &str, detail: impl ToString) -> Msg {
    msg_param(code, "detail", detail)
}

/// The detail is a message too: the window translates it.
pub fn msg_with(code: &str, detail: Msg) -> Msg {
    let mut m = msg(code);
    m.params.insert("detail".into(), serde_json::to_value(detail).unwrap_or_default());
    m
}

/// Text shown as is (from Windows, the network).
pub fn raw(text: impl ToString) -> Msg {
    msg_param("raw", "text", text)
}

pub fn storage(e: impl ToString) -> Msg {
    msg_detail("err.storage", e)
}

/// A failed Steam request; the full reason goes to the log.
pub fn steam(e: SteamError) -> Msg {
    log::warn!("steam request failed: {e}");
    match e {
        SteamError::Unauthorized => msg("err.session_expired"),
        SteamError::RateLimited => msg("err.rate_limited"),
        SteamError::Refused { .. } => msg_detail("err.steam_refused", e),
        SteamError::Network(_) => msg("err.network"),
        SteamError::BadAnswer(_) => msg("err.bad_answer"),
        SteamError::BadToken => msg("err.bad_token"),
    }
}
