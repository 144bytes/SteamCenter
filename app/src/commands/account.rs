//! Signing in to Steam (the sign-in window, or a token pasted by hand) and signing out.

use steamcenter_steam::Token;
use tauri::AppHandle;

use crate::error::{steam, Msg};
use crate::platform::login;
use crate::services::account;

/// Open Steam's sign-in window; the result arrives as a state change (and a notice).
#[tauri::command(async)]
pub fn sign_in(app: AppHandle) -> Result<(), Msg> {
    login::open(&app, false)
}

/// The access token pasted by hand (`webapi_token` from the store, or the `steamLoginSecure` cookie): for when the
/// sign-in window can't be used.
#[tauri::command(async)]
pub fn sign_in_token(token: String, app: AppHandle) -> Result<(), Msg> {
    let token = Token::parse(&token).map_err(steam)?;
    account::complete(&app, token)
}

#[tauri::command(async)]
pub fn sign_out(app: AppHandle) -> Result<(), Msg> {
    account::sign_out(&app)
}
