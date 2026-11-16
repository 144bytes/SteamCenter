//! The game library and its privacy: games marked private are hidden from everyone but the owner (as if not owned).
//! Every change first saves the current private list, so it can be put back.

use std::collections::{BTreeSet, HashMap};

use steamcenter_steam::icon_url;
use tauri::{AppHandle, State};

use crate::error::{msg, steam, storage, Msg};
use crate::models::{GameTags, GameView, StoreTagsView, TagView};
use crate::services::account::client;
use crate::services::jobs;
use crate::state::Shared;
use crate::storage::store::{self, BackupInfo};

#[tauri::command(async)]
pub fn get_library(state: State<Shared>) -> Result<Vec<GameView>, Msg> {
    let c = client(&state)?;
    let private = c.private_apps().map_err(steam)?;
    let mut games: Vec<GameView> = c
        .owned_games()
        .map_err(steam)?
        .into_iter()
        .map(|g| GameView {
            private: private.contains(&g.appid),
            icon: icon_url(g.appid, &g.icon),
            name: if g.name.is_empty() { format!("App {}", g.appid) } else { g.name },
            appid: g.appid,
            playtime: g.playtime,
            last_played: g.last_played,
        })
        .collect();
    // Private games the library list leaves out: named by the store.
    let missing: Vec<u32> = private.iter().copied().filter(|id| !games.iter().any(|g| g.appid == *id)).collect();
    if !missing.is_empty() {
        let names = c.app_names(&missing).unwrap_or_else(|e| {
            log::warn!("no store names for {} private apps: {e}", missing.len());
            Default::default()
        });
        games.extend(missing.into_iter().map(|appid| GameView {
            name: names.get(&appid).cloned().unwrap_or_else(|| format!("App {appid}")),
            appid,
            icon: String::new(),
            playtime: 0,
            last_played: 0,
            private: true,
        }));
    }
    games.sort_by_cached_key(|g| g.name.to_lowercase());
    Ok(games)
}

/// Mark the games private (`private`) or not. Only the ones whose mark changes are sent; returns their count.
#[tauri::command(async)]
pub fn set_private(appids: Vec<u32>, private: bool, app: AppHandle, state: State<Shared>) -> Result<usize, Msg> {
    let c = client(&state)?;
    let job = jobs::start(&app, "library")?;
    let current = c.private_apps().map_err(steam)?;
    let todo: Vec<u32> = appids.into_iter().filter(|id| current.contains(id) != private).collect::<BTreeSet<_>>().into_iter().collect();
    if todo.is_empty() {
        return Ok(0);
    }
    store::save_backup(c.steamid(), &current).map_err(storage)?;
    c.set_private(&todo, private, |done, total| job.progress(done, total)).map_err(steam)?;
    log::info!("{} games marked {}", todo.len(), if private { "private" } else { "public" });
    Ok(todo.len())
}

/// The store's tags and content descriptors of the games, for "make private by rule". Tags come with their names
/// and how many of these games carry each (games the store no longer sells have none).
#[tauri::command(async)]
pub fn get_store_tags(appids: Vec<u32>, state: State<Shared>) -> Result<StoreTagsView, Msg> {
    let c = client(&state)?;
    let games = c.store_items(&appids, true).map_err(steam)?;
    let names = c.tag_names().unwrap_or_else(|e| {
        log::warn!("no store tag names: {e}");
        Default::default()
    });
    let mut counts: HashMap<u32, usize> = HashMap::new();
    for id in games.values().flat_map(|g| &g.tags) {
        *counts.entry(*id).or_default() += 1;
    }
    let mut tags: Vec<TagView> = counts
        .into_iter()
        .filter_map(|(id, count)| Some(TagView { id, name: names.get(&id)?.clone(), count }))
        .collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    let games = games.into_iter().map(|(id, g)| (id, GameTags { tags: g.tags, adult: g.adult, free: g.free })).collect();
    Ok(StoreTagsView { games, tags })
}

#[tauri::command]
pub fn list_backups(state: State<Shared>) -> Vec<BackupInfo> {
    let s = state.lock().unwrap();
    s.account.as_ref().map(|a| store::list_backups(&a.steamid)).unwrap_or_default()
}

/// Make the private list exactly what the backup holds; returns how many games changed.
#[tauri::command(async)]
pub fn restore_backup(name: String, app: AppHandle, state: State<Shared>) -> Result<usize, Msg> {
    let c = client(&state)?;
    let wanted = store::load_backup(c.steamid(), &name).ok_or_else(|| msg("err.backup_missing"))?;
    let job = jobs::start(&app, "library")?;
    let current = c.private_apps().map_err(steam)?;
    let hide: Vec<u32> = wanted.difference(&current).copied().collect();
    let show: Vec<u32> = current.difference(&wanted).copied().collect();
    let total = hide.len() + show.len();
    if total == 0 {
        return Ok(0);
    }
    store::save_backup(c.steamid(), &current).map_err(storage)?;
    c.set_private(&hide, true, |done, _| job.progress(done, total)).map_err(steam)?;
    c.set_private(&show, false, |done, _| job.progress(hide.len() + done, total)).map_err(steam)?;
    log::info!("private list restored from {name}: {} hidden, {} shown", hide.len(), show.len());
    Ok(total)
}
