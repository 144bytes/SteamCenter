//! Signing in to Steam: Steam's own sign-in page in a window of the app. The user types the password, Steam Guard
//! code or scans the QR code there - the app never sees them. Once Steam sets the `steamLoginSecure` cookie, the
//! app trades it for the web session's access token and closes the window.
//!
//! The page is a remote one: the window gets no IPC (see capabilities/default.json). Its cookies stay in the app's
//! browser data, so an expired sign-in is renewed unseen on the next start while Steam still remembers the device.

use std::time::{Duration, Instant};

use steamcenter_steam::Token;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::error::{msg, msg_detail, steam, Msg};
use crate::services::account;
use crate::state::{notice, notify, Shared};
use crate::storage::store::webview_dir;

pub const LABEL: &str = "steam-login";
const START: &str = "https://store.steampowered.com/login/?redir=account%2F&redir_ssl=1";
const COOKIE_SITE: &str = "https://store.steampowered.com/";
/// Unseen renewal gives up after this: Steam wants the password again, the user signs in when they choose to.
const SILENT_LIMIT: Duration = Duration::from_secs(25);
/// A forgotten sign-in window closes by itself.
const LIMIT: Duration = Duration::from_secs(15 * 60);

/// Open the sign-in window (or bring it forward). `silent` - renew an expired sign-in without showing anything.
pub fn open(app: &AppHandle, silent: bool) -> Result<(), Msg> {
    if let Some(w) = app.get_webview_window(LABEL) {
        if !silent {
            let _ = w.show();
            let _ = w.set_focus();
        }
        return Ok(());
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(START.parse().expect("valid URL")))
        .title("Steam - SteamCenter")
        .inner_size(560.0, 780.0)
        .center()
        .visible(!silent)
        .data_directory(webview_dir())
        .build()
        .map_err(|e| msg_detail("err.sign_in", e))?;
    app.state::<Shared>().lock().unwrap().signing_in = true;
    notify(app);
    log::info!("Steam sign-in window opened{}", if silent { " (renewing unseen)" } else { "" });
    let app = app.clone();
    std::thread::spawn(move || watch(app, silent));
    Ok(())
}

/// Wait for the cookie, while the window is open. Runs on its own thread: reading cookies from a command or an
/// event handler deadlocks WebView2.
fn watch(app: AppHandle, silent: bool) {
    let started = Instant::now();
    let found = loop {
        std::thread::sleep(Duration::from_secs(1));
        let Some(w) = app.get_webview_window(LABEL) else { break None };
        if let Some(cookie) = login_cookie(&w) {
            break Some(cookie);
        }
        if (silent && started.elapsed() > SILENT_LIMIT) || started.elapsed() > LIMIT {
            break None;
        }
    };
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
    app.state::<Shared>().lock().unwrap().signing_in = false;
    match found {
        Some(cookie) => {
            // The store's own token for its pages; the cookie's token works too if the store doesn't answer.
            let token = steamcenter_steam::web_token(&cookie).or_else(|e| {
                log::warn!("no web token from the store ({e}), using the cookie's");
                Token::parse(&cookie)
            });
            match token.map_err(steam).and_then(|t| account::complete(&app, t)) {
                Ok(()) => notice(&app, "ok", msg("account.signed_in")),
                Err(m) => notice(&app, "err", m),
            }
        }
        None if silent => {
            log::info!("the sign-in could not be renewed unseen: Steam wants it again");
            notice(&app, "err", msg("err.session_expired"));
        }
        None => log::info!("Steam sign-in window closed without signing in"),
    }
    notify(&app);
}

/// The `steamLoginSecure` cookie of a live session (an expired one, left from before, doesn't count).
fn login_cookie(w: &WebviewWindow) -> Option<String> {
    let cookies = w.cookies_for_url(Url::parse(COOKIE_SITE).ok()?).ok()?;
    let value = cookies.iter().find(|c| c.name() == "steamLoginSecure")?.value().to_string();
    let token = Token::parse(&value).ok()?;
    (!token.expires_within(60)).then_some(value)
}

/// Forget the Steam session of the sign-in window (sign-out): its cookies go, the next sign-in asks for the password.
pub fn forget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(LABEL) {
        let _ = w.close();
    }
    let built = WebviewWindowBuilder::new(app, "steam-forget", WebviewUrl::External("about:blank".parse().expect("valid URL")))
        .visible(false)
        .data_directory(webview_dir())
        .build();
    match built {
        Ok(w) => {
            if let Err(e) = w.clear_all_browsing_data() {
                log::warn!("could not clear the sign-in window's data: {e}");
            }
            let _ = w.close();
        }
        Err(e) => log::warn!("could not clear the sign-in window's data: {e}"),
    }
}
