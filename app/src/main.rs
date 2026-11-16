// A regular app window without a console (in debug builds the console stays).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! SteamCenter: the entry point. Builds the state, opens the windows and registers the IPC commands.
//!
//! Layers: `commands` (IPC) -> `services` (what the program does) -> `platform` (Windows, windows of the app)
//! and `storage` (disk). The Steam Web API itself is the `steamcenter-steam` crate.
//! Everything the program keeps is in `%APPDATA%\SteamCenter\` - nothing next to the exe, nothing elsewhere.

mod commands;
mod error;
mod models;
mod platform;
mod services;
mod state;
mod storage;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use commands::update::AFTER_UPDATE;
use platform::system::{since_start_ms, wait_exit};
use platform::{tray, window};
use state::{AppState, Shared};
use storage::{logs, store};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // A copy that "runs" a game for card drops (see platform/idle.rs): no window, nothing else.
    if args.get(1).map(String::as_str) == Some(platform::idle::IDLE_ARG) {
        std::process::exit(platform::idle::child(&args));
    }
    let after_update = args.iter().any(|a| a == AFTER_UPDATE);
    if let Some(pid) = args.iter().position(|a| a == AFTER_UPDATE).and_then(|i| args.get(i + 1)?.parse().ok()) {
        wait_exit(pid, Duration::from_secs(15));
    }
    let (account, token) = services::account::load();
    let shared: Shared = Arc::new(Mutex::new(AppState {
        settings: store::load_settings(),
        account,
        token,
        after_update,
        ..Default::default()
    }));
    tauri::Builder::default()
        // One copy of the program: starting it again brings the open window forward. Goes first, so a second
        // copy exits before anything below runs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| tray::show_main(app)))
        .manage(shared)
        .setup(|app| {
            logs::init();
            log::info!("SteamCenter {VERSION} started: {} ms after process start", since_start_ms());
            std::thread::spawn(|| steamcenter_update::remove_previous_exe(&store::update_dir()));
            logs::attach(app.handle().clone());
            let main = window::create_main(app.handle())?;
            window::restore(&main);
            tray::create(app.handle())?;
            services::account::refresh_on_start(app.handle());
            log::info!("window shown: {} ms after process start", since_start_ms());
            Ok(())
        })
        .on_window_event(tray::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::app::get_state,
            commands::app::save_settings,
            commands::app::get_logs,
            commands::app::open_logs,
            commands::app::save_report,
            commands::app::open_url,
            commands::app::open_window,
            commands::app::quit,
            commands::account::sign_in,
            commands::account::sign_in_token,
            commands::account::sign_out,
            commands::library::get_library,
            commands::library::set_private,
            commands::library::get_store_tags,
            commands::library::list_backups,
            commands::library::restore_backup,
            commands::profile::get_profile,
            commands::profile::get_appearance,
            commands::profile::set_item,
            commands::profile::set_theme,
            commands::cards::load_cards,
            commands::cards::idle_start,
            commands::cards::idle_stop,
            commands::friends::get_friends,
            commands::friends::remove_friends,
            commands::security::get_sessions,
            commands::wishlist::get_wishlist,
            commands::wishlist::remove_wishlist,
            commands::update::check_update,
            commands::update::install_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the SteamCenter window");
}
