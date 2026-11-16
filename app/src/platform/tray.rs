//! The tray icon. Left click opens the window; right click shows our own menu - a small borderless window
//! drawn by the frontend in the app's style (the native tray menu can't be styled). The window's close
//! button hides it to the tray when "Minimize to tray on close" is on, else quits like "Quit" in the menu.
//!
//! Like a native menu, ours never takes focus: taking it would close the taskbar's hidden icons panel the icon
//! sits in. So it can't close on focus loss either - while it is open, a mouse hook closes it on any press
//! outside it.

use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::OnceLock;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, Window, WindowEvent};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, GetWindowRect, PostQuitMessage, SetWindowsHookExW, UnhookWindowsHookEx, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL,
    WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN,
};

use crate::storage::store::webview_dir;

const MENU: &str = "tray-menu";
/// The menu's size in logical pixels: two items, as `.tray-menu` in the UI styles draws them.
const MENU_SIZE: (f64, f64) = (180.0, 80.0);

static APP: OnceLock<AppHandle> = OnceLock::new();
/// The menu window's handle: the hook reads its place directly, without a trip to the main thread.
static MENU_HWND: AtomicIsize = AtomicIsize::new(0);
static OPEN: AtomicBool = AtomicBool::new(false);
/// The mouse hook is installed (its thread runs).
static WATCHING: AtomicBool = AtomicBool::new(false);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let _ = APP.set(app.clone());
    // Made once and kept hidden; the frontend draws it for `#tray-menu`.
    let menu = WebviewWindowBuilder::new(app, MENU, WebviewUrl::App("index.html#tray-menu".into()))
        .title("SteamCenter")
        .inner_size(MENU_SIZE.0, MENU_SIZE.1)
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .always_on_top(true)
        .focusable(false)
        .focused(false)
        .visible(false)
        .data_directory(webview_dir())
        .build()?;
    MENU_HWND.store(menu.hwnd()?.0 as isize, Ordering::SeqCst);
    let mut tray = TrayIconBuilder::new().tooltip("SteamCenter").show_menu_on_left_click(false).on_tray_icon_event(|t, e| {
        if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, position, .. } = e {
            match button {
                MouseButton::Left => show_main(t.app_handle()),
                MouseButton::Right => show_menu(t.app_handle(), position),
                _ => {}
            }
        }
    });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Bring the main window back (from the tray, minimized or behind other windows).
pub fn show_main(app: &AppHandle) {
    hide_menu(app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// The menu above and to the left of the pointer (the taskbar is usually at the bottom right), inside its monitor.
fn show_menu(app: &AppHandle, at: PhysicalPosition<f64>) {
    let (Some(menu), Ok(Some(mon))) = (app.get_webview_window(MENU), app.monitor_from_point(at.x, at.y)) else { return };
    let (w, h) = (MENU_SIZE.0 * mon.scale_factor(), MENU_SIZE.1 * mon.scale_factor());
    let (left, top) = (mon.position().x as f64, mon.position().y as f64);
    let x = if at.x - w >= left { at.x - w } else { at.x };
    let y = if at.y - h >= top { at.y - h } else { at.y };
    let _ = menu.set_position(PhysicalPosition::new(x, y));
    let _ = menu.show();
    OPEN.store(true, Ordering::SeqCst);
    if !WATCHING.swap(true, Ordering::SeqCst) {
        std::thread::spawn(watch_clicks);
    }
}

fn hide_menu(app: &AppHandle) {
    OPEN.store(false, Ordering::SeqCst);
    if let Some(m) = app.get_webview_window(MENU) {
        let _ = m.hide();
    }
}

/// The mouse hook's thread: lives while the menu is open (the hook leaves at the first mouse event after it closes).
fn watch_clicks() {
    unsafe {
        let Ok(hook) = SetWindowsHookExW(WH_MOUSE_LL, Some(on_mouse), None, 0) else {
            WATCHING.store(false, Ordering::SeqCst);
            return;
        };
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {}
        let _ = UnhookWindowsHookEx(hook);
    }
}

unsafe extern "system" fn on_mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if !OPEN.load(Ordering::SeqCst) {
        // Closed: leave - unless it was opened again just now and counts on this thread (`show_menu` saw it running).
        WATCHING.store(false, Ordering::SeqCst);
        if !OPEN.load(Ordering::SeqCst) || WATCHING.swap(true, Ordering::SeqCst) {
            PostQuitMessage(0);
        }
    } else if let (true, Some(app)) = (code >= 0, APP.get()) {
        let press = matches!(wparam.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN);
        if press && !inside_menu((*(lparam.0 as *const MSLLHOOKSTRUCT)).pt) {
            hide_menu(app);
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn inside_menu(pt: POINT) -> bool {
    let mut r = RECT::default();
    let hwnd = HWND(MENU_HWND.load(Ordering::SeqCst) as *mut _);
    unsafe { GetWindowRect(hwnd, &mut r) }.is_ok() && (r.left..r.right).contains(&pt.x) && (r.top..r.bottom).contains(&pt.y)
}

/// Closing the main window hides it to the tray, or quits when "Minimize to tray on close" is off; the menu only hides.
/// The Steam sign-in window closes for real.
pub fn on_window_event(w: &Window, e: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = e else { return };
    match w.label() {
        "main" => {
            api.prevent_close();
            if w.state::<crate::state::Shared>().lock().unwrap().settings.close_to_tray {
                crate::platform::window::save(w);
                let _ = w.hide();
            } else {
                crate::commands::app::quit_app(w.app_handle(), false);
            }
        }
        MENU => {
            api.prevent_close();
            hide_menu(w.app_handle());
        }
        _ => {}
    }
}
