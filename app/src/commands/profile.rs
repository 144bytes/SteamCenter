//! The profile: its summary and recent games, and its look - background, mini-profile, avatar frame, animated avatar
//! and theme, chosen from the items the account owns.

use steamcenter_steam::ItemKind;
use tauri::State;

use crate::error::{msg, steam, Msg};
use crate::models::{AppearanceView, ProfileView};
use crate::services::account::client;
use crate::state::Shared;

#[tauri::command(async)]
pub fn get_profile(state: State<Shared>) -> Result<ProfileView, Msg> {
    let c = client(&state)?;
    let summary = c.summary().map_err(steam)?;
    // Extras: the profile shows without them.
    let level = c.level().unwrap_or_else(|e| {
        log::warn!("no Steam level: {e}");
        0
    });
    let recent = c.recent_games().unwrap_or_else(|e| {
        log::warn!("no recent games: {e}");
        Vec::new()
    });
    Ok(ProfileView { summary, level, recent })
}

#[tauri::command(async)]
pub fn get_appearance(state: State<Shared>) -> Result<AppearanceView, Msg> {
    let c = client(&state)?;
    let slots = c.item_slots().map_err(steam)?;
    let (themes, theme) = c.themes().unwrap_or_else(|e| {
        log::warn!("no profile themes: {e}");
        (Vec::new(), String::new())
    });
    Ok(AppearanceView { slots, themes, theme })
}

/// Wear an owned item of `kind`; an empty `id` takes the worn one off.
#[tauri::command(async)]
pub fn set_item(kind: String, id: String, state: State<Shared>) -> Result<(), Msg> {
    let kind = ItemKind::parse(&kind).ok_or_else(|| msg("err.bad_request"))?;
    client(&state)?.set_item(kind, &id).map_err(steam)?;
    log::info!("profile {}: {}", kind.key(), if id.is_empty() { "removed" } else { "changed" });
    Ok(())
}

/// An empty `id` - the default theme.
#[tauri::command(async)]
pub fn set_theme(id: String, state: State<Shared>) -> Result<(), Msg> {
    client(&state)?.set_theme(&id).map_err(steam)?;
    log::info!("profile theme: {}", if id.is_empty() { "default" } else { &id });
    Ok(())
}
