//! The main window keeps its size and place between runs (%APPDATA%\RdpCenter\window.json).
//! Windows itself keeps the window's normal place even while it is maximized or minimized
//! (GetWindowPlacement), so that is what is saved and restored.

use tauri::window::Color;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, Window};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowPlacement, SetWindowPlacement, SHOW_WINDOW_CMD, SW_SHOWMAXIMIZED, SW_SHOWMINIMIZED, SW_SHOWNORMAL,
    WINDOWPLACEMENT, WPF_RESTORETOMAXIMIZED,
};

use crate::storage::store::{self, WindowGeometry};

/// The main window, hidden until `restore` puts it in place. Made here rather than in tauri.conf.json: every window
/// of the app must keep its browser data in the app's folder, and only the builder can say where.
pub fn create_main(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .title("SteamCenter")
        .inner_size(1080.0, 720.0)
        .min_inner_size(860.0, 560.0)
        .background_color(Color(0x1b, 0x1b, 0x1a, 0xff))
        .center()
        .visible(false)
        .data_directory(store::webview_dir())
        .build()
}

fn hwnd(w: &Window) -> Option<HWND> {
    w.hwnd().ok().map(|h| HWND(h.0 as _))
}

/// Put the window where it was last time - if that place is still on a monitor - and show it.
/// The window starts hidden, so it never flashes at the default size first.
pub fn restore(w: &WebviewWindow) {
    let saved = store::load_window().filter(|g| on_screen(w, g));
    if let (Some(g), Some(h)) = (saved, hwnd(&w.as_ref().window())) {
        let place = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            showCmd: if g.maximized { SW_SHOWMAXIMIZED.0 as u32 } else { SW_SHOWNORMAL.0 as u32 },
            rcNormalPosition: RECT { left: g.x, top: g.y, right: g.x + g.width as i32, bottom: g.y + g.height as i32 },
            ..Default::default()
        };
        unsafe {
            let _ = SetWindowPlacement(h, &place);
        }
    }
    let _ = w.show();
    let _ = w.set_focus();
}

/// A monitor may be unplugged or its resolution changed since: the title bar must be on one of them.
fn on_screen(w: &WebviewWindow, g: &WindowGeometry) -> bool {
    let (x, y) = (g.x + 100, g.y + 15);
    w.available_monitors().unwrap_or_default().iter().any(|m| {
        let (p, s) = (m.position(), m.size());
        x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
    })
}

/// The program is ending: remember the window's place if it is on screen (hidden to the tray, it was
/// remembered when it was hidden).
pub fn save_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main").filter(|w| w.is_visible().unwrap_or(false)) {
        save(&w.as_ref().window());
    }
}

pub fn save(w: &Window) {
    let Some(h) = hwnd(w) else { return };
    let mut place = WINDOWPLACEMENT { length: std::mem::size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
    if unsafe { GetWindowPlacement(h, &mut place) }.is_err() {
        return;
    }
    let cmd = SHOW_WINDOW_CMD(place.showCmd as i32);
    let maximized = cmd == SW_SHOWMAXIMIZED || (cmd == SW_SHOWMINIMIZED && place.flags.contains(WPF_RESTORETOMAXIMIZED));
    let r = place.rcNormalPosition;
    let g = WindowGeometry { x: r.left, y: r.top, width: (r.right - r.left) as u32, height: (r.bottom - r.top) as u32, maximized };
    if let Err(e) = store::save_window(&g) {
        log::warn!("could not save the window position: {e}");
    }
}
