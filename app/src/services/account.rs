//! The signed-in Steam account: loading it at start, finishing a sign-in, signing out, and the API client for it.

use steamcenter_steam::{Client, SteamError, Token};
use tauri::{AppHandle, Manager};

use crate::error::{msg, steam, storage, Msg};
use crate::platform::login;
use crate::state::{notify, Shared};
use crate::storage::store::{self, Account, Sealed};

/// Renew the sign-in when it has less than this left: an operation must not run out of it halfway.
const RENEW_BEFORE: u64 = 60 * 60;

/// The account saved on disk and its token. A token that can't be opened (another Windows user, a damaged file)
/// leaves the account signed out.
pub fn load() -> (Option<Account>, Option<Token>) {
    let Some(account) = store::load_account() else { return (None, None) };
    let token = account.token.open().ok().and_then(|t| Token::parse(&t).ok()).filter(|t| t.steamid == account.steamid);
    (Some(account), token)
}

/// At start: an expired (or nearly) sign-in is renewed unseen through the sign-in window's remembered session.
pub fn refresh_on_start(app: &AppHandle) {
    let s = app.state::<Shared>();
    let s = s.lock().unwrap();
    if s.account.is_some() && s.token.is_none() {
        log::warn!("the saved Steam sign-in can't be opened (another Windows user or a damaged file)");
    }
    let renew = s.account.is_some() && s.token.as_ref().is_none_or(|t| t.expires_within(RENEW_BEFORE));
    drop(s);
    if renew {
        if let Err(e) = login::open(app, true) {
            log::warn!("could not renew the Steam sign-in: {e:?}");
        }
    }
}

/// A new token from the sign-in: ask Steam for the profile (its name and avatar) and remember it. A token Steam
/// refuses is an error; a profile that only failed to load leaves the name as the SteamID until the next sign-in.
pub fn complete(app: &AppHandle, token: Token) -> Result<(), Msg> {
    let language = app.state::<Shared>().lock().unwrap().settings.language.clone();
    let (name, avatar) = match Client::new(token.clone(), &language).summary() {
        Ok(s) => (s.name, s.avatar),
        Err(e @ (SteamError::Unauthorized | SteamError::BadToken)) => return Err(steam(e)),
        Err(e) => {
            log::warn!("signed in, but the profile did not load: {e}");
            (token.steamid.clone(), String::new())
        }
    };
    let account = Account { steamid: token.steamid.clone(), name, avatar, token: Sealed::seal(&token.value).map_err(storage)? };
    store::save_account(Some(&account)).map_err(storage)?;
    log::info!("signed in to Steam as {} ({}), until {}", account.name, account.steamid, expiry(&token));
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap();
    s.account = Some(account);
    s.token = Some(token);
    drop(s);
    notify(app);
    Ok(())
}

fn expiry(t: &Token) -> String {
    chrono::DateTime::from_timestamp(t.expires as i64, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "?".into())
}

pub fn sign_out(app: &AppHandle) -> Result<(), Msg> {
    store::save_account(None).map_err(storage)?;
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap();
    s.account = None;
    s.token = None;
    s.idle.games.clear();
    s.idle.loaded = false;
    drop(s);
    super::idle::stop();
    login::forget(app);
    log::info!("signed out of Steam");
    notify(app);
    Ok(())
}

/// The API client of the signed-in account; an expired sign-in is an error the window offers to renew.
pub fn client(state: &Shared) -> Result<Client, Msg> {
    let s = state.lock().unwrap();
    match &s.token {
        Some(t) if !t.expires_within(30) => Ok(Client::new(t.clone(), &s.settings.language)),
        Some(_) => Err(msg("err.session_expired")),
        None if s.account.is_some() => Err(msg("err.session_expired")),
        None => Err(msg("err.signed_out")),
    }
}
