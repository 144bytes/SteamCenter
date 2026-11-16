//! Trading cards: the games with drops left, and idling them ("turbo"). The page reads both from the state.

use tauri::AppHandle;

use crate::error::Msg;
use crate::services::idle;

/// Read the badges again.
#[tauri::command(async)]
pub fn load_cards(app: AppHandle) -> Result<(), Msg> {
    idle::load(&app).map(drop)
}

#[tauri::command]
pub fn idle_start(app: AppHandle) -> Result<(), Msg> {
    idle::start(&app)
}

#[tauri::command]
pub fn idle_stop() {
    idle::stop();
}
