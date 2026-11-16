//! The program's state in memory, shared by the commands and the background threads.

use std::sync::{Arc, Mutex};

use serde_json::json;
use steamcenter_steam::Token;
use tauri::{AppHandle, Emitter};

use crate::error::Msg;
use crate::storage::store::{Account, Settings};

#[derive(Default)]
pub struct AppState {
    pub settings: Settings,
    /// The signed-in Steam account (kept on disk) and its opened token.
    pub account: Option<Account>,
    pub token: Option<Token>,
    /// The Steam sign-in window is open (or renewing the sign-in unseen).
    pub signing_in: bool,
    /// A long operation that changes the account ("library", "wishlist", "friends"): one at a time.
    pub job: Option<&'static str>,
    /// Started by an update: the window doesn't check for one again (a release whose file lags behind its tag would
    /// restart the program in a loop).
    pub after_update: bool,
}

pub type Shared = Arc<Mutex<AppState>>;

/// Tell the window the state changed: it reads it again.
pub fn notify(app: &AppHandle) {
    let _ = app.emit("state", ());
}

/// A message the window shows by itself (not the answer to a command): `kind` - "ok" or "err".
pub fn notice(app: &AppHandle, kind: &str, m: Msg) {
    let _ = app.emit_to("main", "notice", json!({ "kind": kind, "msg": m }));
}
