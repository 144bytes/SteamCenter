//! The program itself: its state and settings, the log, the support report, links, the tray menu's "Open" and "Quit".

use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};

use crate::error::{msg, storage, Msg};
use crate::models::AccountView;
use crate::platform::system::{open_url as shell_open, since_start_ms};
use crate::platform::{tray, window};
use crate::services::{idle, jobs, report};
use crate::state::{notify, Shared};
use crate::storage::store::{self, Settings};
use crate::storage::logs;
use crate::VERSION;

#[tauri::command]
pub fn get_state(state: State<Shared>) -> Value {
    // The window's first read: its contents are on screen next. Slow starts show in the log which step took the time.
    static FIRST: AtomicBool = AtomicBool::new(true);
    if FIRST.swap(false, Ordering::Relaxed) {
        log::info!("window reads its data: {} ms after process start", since_start_ms());
    }
    let s = state.lock().unwrap();
    let account = s.account.as_ref().map(|a| AccountView {
        steamid: a.steamid.clone(),
        name: a.name.clone(),
        avatar: a.avatar.clone(),
        expires: s.token.as_ref().filter(|t| !t.expires_within(30)).map_or(0, |t| t.expires),
    });
    json!({
        "version": VERSION,
        "settings": s.settings,
        "account": account,
        "signing_in": s.signing_in,
        "job": s.job,
        "idle": s.idle,
        // "Check for updates" by itself at startup (when the setting is on); never in a debug build, which it would
        // replace with the release.
        "update_on_start": !s.after_update && !cfg!(debug_assertions),
    })
}

#[tauri::command]
pub fn save_settings(settings: Settings, state: State<Shared>, app: AppHandle) -> Result<(), Msg> {
    let mut s = state.lock().unwrap();
    s.settings = settings;
    store::save_settings(&s.settings).map_err(storage)?;
    drop(s);
    notify(&app);
    Ok(())
}

#[tauri::command]
pub fn get_logs() -> Vec<logs::LogLine> {
    logs::recent()
}

/// Open the log folder in Explorer.
#[tauri::command]
pub fn open_logs() -> Result<(), Msg> {
    std::process::Command::new("explorer.exe").arg(store::log_dir()).spawn().map_err(storage)?;
    Ok(())
}

/// "Save report": ask where to save it and write a .txt the user can send. Returns the path, None - cancelled.
/// `filter` - the name of the file type in the dialog, in the UI language.
#[tauri::command(async)]
pub fn save_report(filter: String, window: tauri::WebviewWindow, state: State<Shared>) -> Result<Option<String>, Msg> {
    // The log is read after the state is released: it may be megabytes.
    let mut text = report::summary(&state.lock().unwrap());
    text.push_str(&report::log_text().replace("\r\n", "\n"));
    let owner = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let name = format!("SteamCenter-report-{}.txt", chrono::Local::now().format("%Y-%m-%d_%H-%M"));
    let Some(path) = report::ask_path(owner, &name, &filter) else { return Ok(None) };
    std::fs::write(&path, text.replace('\n', "\r\n")).map_err(storage)?;
    log::info!("report saved: {}", path.display());
    Ok(Some(path.display().to_string()))
}

/// A Steam page in the default browser (the profile, its settings). Only Steam's own sites.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), Msg> {
    const ALLOWED: [&str; 2] = ["https://steamcommunity.com/", "https://store.steampowered.com/"];
    if !ALLOWED.iter().any(|p| url.starts_with(p)) {
        return Err(msg("err.bad_request"));
    }
    shell_open(&url).map_err(storage)
}

/// "Open" in the tray menu.
#[tauri::command]
pub fn open_window(app: AppHandle) {
    tray::show_main(&app);
}

/// "Quit" in the tray menu. While an operation changes the account the main window asks first (`force` - it did).
#[tauri::command]
pub fn quit(force: bool, app: AppHandle) {
    quit_app(&app, force);
}

pub fn quit_app(app: &AppHandle, force: bool) {
    if !force && jobs::running(app) {
        tray::show_main(app);
        let _ = app.emit("quit-requested", ());
        return;
    }
    idle::stop();
    window::save_main(app);
    app.exit(0);
}
