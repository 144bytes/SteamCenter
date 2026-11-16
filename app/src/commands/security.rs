//! Security: the devices and browsers signed in to the account.

use steamcenter_steam::Session;
use tauri::State;

use crate::error::{steam, Msg};
use crate::services::account::client;
use crate::state::Shared;

#[tauri::command(async)]
pub fn get_sessions(state: State<Shared>) -> Result<Vec<Session>, Msg> {
    client(&state)?.sessions().map_err(steam)
}
