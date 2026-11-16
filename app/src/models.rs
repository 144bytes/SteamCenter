//! What the window gets: the shapes of the data crossing the IPC boundary.

use std::collections::HashMap;

use serde::Serialize;
use steamcenter_steam::{ItemSlot, PlayerSummary, RecentGame, Theme};

#[derive(Serialize)]
pub struct AccountView {
    pub steamid: String,
    pub name: String,
    pub avatar: String,
    /// When the sign-in expires (unix time); 0 - it has to be renewed.
    pub expires: u64,
}

#[derive(Serialize)]
pub struct GameView {
    pub appid: u32,
    pub name: String,
    pub icon: String,
    /// Minutes.
    pub playtime: u64,
    pub last_played: u64,
    pub private: bool,
}

#[derive(Serialize)]
pub struct GameTags {
    pub tags: Vec<u32>,
    pub adult: bool,
    pub free: bool,
}

#[derive(Serialize)]
pub struct TagView {
    pub id: u32,
    pub name: String,
    /// How many of the games carry it.
    pub count: usize,
}

#[derive(Serialize)]
pub struct StoreTagsView {
    pub games: HashMap<u32, GameTags>,
    /// Most common first.
    pub tags: Vec<TagView>,
}

#[derive(Serialize)]
pub struct ProfileView {
    pub summary: PlayerSummary,
    pub level: u32,
    pub recent: Vec<RecentGame>,
}

#[derive(Serialize)]
pub struct AppearanceView {
    pub slots: Vec<ItemSlot>,
    pub themes: Vec<Theme>,
    pub theme: String,
}

#[derive(Serialize)]
pub struct WishView {
    pub appid: u32,
    pub name: String,
    pub image: String,
    pub added: u64,
    pub priority: u64,
}
