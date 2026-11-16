//! The wishlist: what is on it, and taking games off it in bulk.

use steamcenter_steam::capsule_url;
use tauri::{AppHandle, State};

use crate::error::{steam, Msg};
use crate::models::WishView;
use crate::services::account::client;
use crate::services::jobs;
use crate::state::Shared;

#[tauri::command(async)]
pub fn get_wishlist(state: State<Shared>) -> Result<Vec<WishView>, Msg> {
    let c = client(&state)?;
    let items = c.wishlist().map_err(steam)?;
    let ids: Vec<u32> = items.iter().map(|w| w.appid).collect();
    let names = c.app_names(&ids).unwrap_or_else(|e| {
        log::warn!("no store names for the wishlist: {e}");
        Default::default()
    });
    let mut list: Vec<WishView> = items
        .into_iter()
        .map(|w| WishView {
            name: names.get(&w.appid).cloned().unwrap_or_else(|| format!("App {}", w.appid)),
            image: capsule_url(w.appid),
            appid: w.appid,
            added: w.added,
            priority: w.priority,
        })
        .collect();
    // The user's own order first (0 - not ranked goes last), then the newest.
    list.sort_by_key(|w| (w.priority == 0, w.priority, std::cmp::Reverse(w.added)));
    Ok(list)
}

/// Returns how many games were taken off.
#[tauri::command(async)]
pub fn remove_wishlist(appids: Vec<u32>, app: AppHandle, state: State<Shared>) -> Result<usize, Msg> {
    let c = client(&state)?;
    let job = jobs::start(&app, "wishlist")?;
    c.remove_from_wishlist(&appids, |done, total| job.progress(done, total)).map_err(steam)?;
    log::info!("{} games taken off the wishlist", appids.len());
    Ok(appids.len())
}
