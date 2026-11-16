//! Updating the program from GitHub Releases: checking, downloading, swapping the exe and restarting.
//! The button in Settings and the check at startup are the same two commands.

use std::sync::atomic::{AtomicBool, Ordering};

use steamcenter_update::UpdateError;
use tauri::AppHandle;

use crate::error::{msg, msg_detail, msg_with, raw, Msg};
use crate::platform::window;
use crate::services::jobs;
use crate::storage::store;

/// Command-line flag of the restart after an update: `--after-update <pid of the previous process>`.
pub const AFTER_UPDATE: &str = "--after-update";

/// One download at a time (the window may have been left and the button pressed again, or the check at startup is
/// already downloading).
static UPDATING: AtomicBool = AtomicBool::new(false);

/// "Check for updates": the newer version published on GitHub, or None when this one is the latest.
#[tauri::command(async)]
pub fn check_update() -> Result<Option<String>, Msg> {
    let r = steamcenter_update::latest().map_err(update_error)?;
    Ok((r.version > steamcenter_update::Version::current()).then(|| r.version.to_string()))
}

/// Download the latest version into the app's folder, swap the files and restart.
/// Not while an operation changes the account: the restart would cut it halfway.
#[tauri::command(async)]
pub fn install_update(app: AppHandle) -> Result<(), Msg> {
    if jobs::running(&app) {
        return Err(msg("err.update_busy"));
    }
    if UPDATING.swap(true, Ordering::SeqCst) {
        return Err(msg("err.update_running"));
    }
    let r = steamcenter_update::latest().map_err(update_error).and_then(|release| download_update(&app, release));
    if r.is_err() {
        UPDATING.store(false, Ordering::SeqCst);
    }
    r
}

/// The full reason goes to the log.
fn update_error(e: UpdateError) -> Msg {
    log::warn!("update failed: {e}");
    let detail = match e {
        UpdateError::NoRelease => msg("detail.no_release"),
        UpdateError::Unreachable(_) => msg("detail.github_unreachable"),
        UpdateError::BadRelease(_) => msg("detail.bad_release"),
        UpdateError::Download(_) => msg("detail.download"),
        UpdateError::File(e) => raw(e),
    };
    msg_with("err.update", detail)
}

fn download_update(app: &AppHandle, r: steamcenter_update::Release) -> Result<(), Msg> {
    let dir = store::update_dir();
    let new = dir.join(format!("{}.new", steamcenter_update::ASSET));
    r.download_to(steamcenter_update::ASSET, &new).map_err(|e| {
        let _ = std::fs::remove_file(&new);
        update_error(e)
    })?;
    // An operation started while it was downloading: the restart would cut it - next time, then.
    if jobs::running(app) {
        let _ = std::fs::remove_file(&new);
        log::info!("update to {} postponed: an operation is running", r.version);
        return Err(msg("err.update_busy"));
    }
    let exe = steamcenter_update::replace_running_exe(&new, &dir).map_err(|e| {
        let _ = std::fs::remove_file(&new);
        update_error(e)
    })?;
    log::info!("updating to {}: restarting", r.version);
    // The new process waits for this one to exit: otherwise it would see a running copy and quit.
    std::process::Command::new(&exe)
        .args([AFTER_UPDATE, &std::process::id().to_string()])
        .spawn()
        .map_err(|e| msg_detail("err.update", e))?;
    window::save_main(app);
    app.exit(0);
    Ok(())
}
