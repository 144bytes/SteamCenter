//! Idling for trading cards, "turbo" - the fast mode of Idle Master Extended: up to 30 games run together for
//! 5 minutes, then, after a 5-second pause, each of them runs alone for 5 seconds (starting a game is what makes
//! Steam check for a drop). Then the badges are read again and the cycle repeats with the games that still have
//! drops, until none do or the user stops it.
//!
//! The games "run" as copies of this program (platform/idle.rs); the Steam client must be running and signed in to
//! the same account. Games with Valve Anti-Cheat are left out when the setting says so.

use std::collections::HashMap;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use steamcenter_steam::{capsule_url, now};
use tauri::{AppHandle, Manager};

use crate::error::{msg, steam, Msg};
use crate::platform::idle::{self as process, EXIT_NO_STEAM, EXIT_NOT_RUNNING};
use crate::platform::login;
use crate::state::{notice, notify, Shared};

/// Steam lets an account run at most 32 games at once.
const MAX_GAMES: usize = 30;
const TOGETHER: Duration = Duration::from_secs(5 * 60);
const PAUSE: Duration = Duration::from_secs(5);
const SOLO: Duration = Duration::from_secs(5);
/// How long a started copy gets to connect to Steam before its failure counts.
const START_CHECK: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
pub struct CardView {
    pub appid: u32,
    pub name: String,
    pub cards: u32,
    pub hours: f64,
    pub vac: bool,
    pub image: String,
}

/// What the Cards page shows.
#[derive(Debug, Clone, Default, Serialize)]
pub struct IdleState {
    pub running: bool,
    /// "loading" (reading the badges), "together", "pause", "solo"; empty - not running.
    pub phase: &'static str,
    /// When the phase ends (unix time); 0 - no set end.
    pub phase_ends: u64,
    /// The games running now.
    pub active: Vec<u32>,
    /// The games with drops left, as last read.
    pub games: Vec<CardView>,
    /// Whether `games` was read at all.
    pub loaded: bool,
    pub cycle: u32,
}

static STOP: AtomicBool = AtomicBool::new(false);
static CHILDREN: Mutex<Vec<Child>> = Mutex::new(Vec::new());
/// Whether a game uses VAC, asked once per run of the program.
static VAC: Mutex<Option<HashMap<u32, bool>>> = Mutex::new(None);

fn vac(appid: u32) -> bool {
    if let Some(v) = VAC.lock().unwrap().as_ref().and_then(|m| m.get(&appid)) {
        return *v;
    }
    let v = steamcenter_steam::has_vac(appid).unwrap_or_else(|e| {
        log::warn!("VAC of {appid} unknown: {e}");
        false
    });
    VAC.lock().unwrap().get_or_insert_with(HashMap::new).insert(appid, v);
    std::thread::sleep(Duration::from_millis(250));
    v
}

/// Read the badges: the games with drops left (and whether each uses VAC). Off the window's thread.
pub fn load(app: &AppHandle) -> Result<Vec<CardView>, Msg> {
    let steamid = {
        let s = app.state::<Shared>();
        let s = s.lock().unwrap();
        s.token.as_ref().filter(|t| !t.expires_within(30)).map(|t| t.steamid.clone()).ok_or_else(|| msg("err.session_expired"))?
    };
    let cookie = login::community_cookie(app)?;
    let mut games: Vec<CardView> = steamcenter_steam::card_drops(&steamid, &cookie)
        .map_err(steam)?
        .into_iter()
        .map(|g| CardView { vac: vac(g.appid), image: capsule_url(g.appid), appid: g.appid, name: g.name, cards: g.cards, hours: g.hours })
        .collect();
    games.sort_by(|a, b| b.cards.cmp(&a.cards).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap();
    s.idle.games = games.clone();
    s.idle.loaded = true;
    drop(s);
    notify(app);
    Ok(games)
}

pub fn start(app: &AppHandle) -> Result<(), Msg> {
    if process::steam_dir().is_none() {
        return Err(msg("err.steam_missing"));
    }
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap();
    if s.idle.running {
        return Ok(());
    }
    s.idle.running = true;
    s.idle.phase = "loading";
    s.idle.cycle = 0;
    drop(s);
    STOP.store(false, Ordering::SeqCst);
    log::info!("card idling started");
    notify(app);
    let app = app.clone();
    std::thread::spawn(move || {
        let result = run(&app);
        stop_games();
        let state = app.state::<Shared>();
        let mut s = state.lock().unwrap();
        s.idle.running = false;
        s.idle.phase = "";
        s.idle.phase_ends = 0;
        s.idle.active.clear();
        drop(s);
        match result {
            Ok(true) => {
                log::info!("card idling finished: no drops left");
                notice(&app, "ok", msg("cards.done"));
            }
            Ok(false) => log::info!("card idling stopped"),
            Err(m) => {
                log::warn!("card idling stopped: {}", m.code);
                notice(&app, "err", m);
            }
        }
        notify(&app);
    });
    Ok(())
}

/// Stop idling (the loop ends at its next check; the games stop now).
pub fn stop() {
    STOP.store(true, Ordering::SeqCst);
    stop_games();
}

fn stop_games() {
    for mut c in CHILDREN.lock().unwrap().drain(..) {
        let _ = c.kill();
        let _ = c.wait();
    }
}

fn phase(app: &AppHandle, name: &'static str, length: Duration, active: Vec<u32>) {
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap();
    s.idle.phase = name;
    s.idle.phase_ends = if length.is_zero() { 0 } else { now() + length.as_secs() };
    s.idle.active = active;
    drop(s);
    notify(app);
}

/// Wait, or less if stopped meanwhile; false - stopped.
fn wait(length: Duration) -> bool {
    let end = Instant::now() + length;
    while Instant::now() < end {
        if STOP.load(Ordering::SeqCst) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    !STOP.load(Ordering::SeqCst)
}

/// Run the games; the copies that ended at once (could not reach Steam) are an error when none is left running.
fn run_games(appids: &[u32]) -> Result<(), Msg> {
    for &appid in appids {
        match process::spawn(appid) {
            Ok(c) => CHILDREN.lock().unwrap().push(c),
            Err(e) => log::warn!("could not start idling {appid}: {e}"),
        }
    }
    if !wait(START_CHECK) {
        return Ok(());
    }
    let mut children = CHILDREN.lock().unwrap();
    let mut failed = None;
    children.retain_mut(|c| match c.try_wait() {
        Ok(Some(status)) => {
            failed = status.code();
            false
        }
        _ => true,
    });
    match (children.is_empty(), failed) {
        (true, Some(EXIT_NO_STEAM)) => Err(msg("err.steam_missing")),
        (true, Some(EXIT_NOT_RUNNING)) => Err(msg("err.steam_not_running")),
        (true, _) => Err(msg("err.idle_failed")),
        _ => Ok(()),
    }
}

/// The idling loop. Ok(true) - no drops left, Ok(false) - stopped.
fn run(app: &AppHandle) -> Result<bool, Msg> {
    loop {
        phase(app, "loading", Duration::ZERO, Vec::new());
        let skip_vac = app.state::<Shared>().lock().unwrap().settings.skip_vac;
        let games: Vec<u32> = load(app)?.iter().filter(|g| g.cards > 0 && !(skip_vac && g.vac)).map(|g| g.appid).take(MAX_GAMES).collect();
        if games.is_empty() {
            return Ok(true);
        }
        if STOP.load(Ordering::SeqCst) {
            return Ok(false);
        }
        let cycle = {
            let state = app.state::<Shared>();
            let mut s = state.lock().unwrap();
            s.idle.cycle += 1;
            s.idle.cycle
        };
        log::info!("idling cycle {cycle}: {} games together", games.len());
        phase(app, "together", TOGETHER, games.clone());
        run_games(&games)?;
        if !wait(TOGETHER.saturating_sub(START_CHECK)) {
            return Ok(false);
        }
        stop_games();
        phase(app, "pause", PAUSE, Vec::new());
        if !wait(PAUSE) {
            return Ok(false);
        }
        for &appid in &games {
            phase(app, "solo", SOLO, vec![appid]);
            run_games(&[appid])?;
            if !wait(SOLO.saturating_sub(START_CHECK)) {
                return Ok(false);
            }
            stop_games();
        }
    }
}
