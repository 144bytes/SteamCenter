//! Friends: the list with when each was last online, and removing many at once.

use steamcenter_steam::PlayerSummary;
use tauri::{AppHandle, State};

use crate::error::{steam, Msg};
use crate::services::account::client;
use crate::services::jobs;
use crate::state::Shared;

#[tauri::command(async)]
pub fn get_friends(state: State<Shared>) -> Result<Vec<PlayerSummary>, Msg> {
    let mut friends = client(&state)?.friends().map_err(steam)?;
    friends.sort_by_cached_key(|f| f.name.to_lowercase());
    Ok(friends)
}

/// Returns how many were removed. A removed friend can't be brought back from here (they'd have to accept a new
/// request), so the log keeps who it was.
#[tauri::command(async)]
pub fn remove_friends(steamids: Vec<String>, app: AppHandle, state: State<Shared>) -> Result<usize, Msg> {
    let c = client(&state)?;
    let job = jobs::start(&app, "friends")?;
    c.remove_friends(&steamids, |done, total| job.progress(done, total)).map_err(steam)?;
    log::info!("{} friends removed: {}", steamids.len(), steamids.join(", "));
    Ok(steamids.len())
}
