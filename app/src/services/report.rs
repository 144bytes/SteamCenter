//! Support report: saved as a .txt file the user can send. The full log goes in, so it does not fit the clipboard.
//! No token gets into it.

use std::path::PathBuf;

use windows::core::{w, PCWSTR, PWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::Controls::Dialogs::{
    GetSaveFileNameW, OFN_NOCHANGEDIR, OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Shell::FOLDERID_Desktop;

use crate::platform::system::windows_version;
use crate::platform::win32::{from_wide, known_folder, wide};
use crate::state::AppState;
use crate::VERSION;

/// The log files, oldest first (the previous rotated file, then the current one).
pub fn log_text() -> String {
    let dir = crate::storage::store::log_dir();
    let mut out = String::new();
    for name in ["steamcenter.1.log", "steamcenter.log"] {
        if let Ok(t) = std::fs::read_to_string(dir.join(name)) {
            out.push_str(&t);
        }
    }
    out
}

/// The standard Windows "Save as" dialog on its own thread (it needs COM). None - the user cancelled.
/// `owner` is the main window handle: the dialog is modal to it. `filter_name` - the file type name in the UI language.
pub fn ask_path(owner: isize, default_name: &str, filter_name: &str) -> Option<PathBuf> {
    let default_name = default_name.to_string();
    // `wide` adds the final \0 that ends the list.
    let filter = wide(&format!("{filter_name} (*.txt)\0*.txt\0"));
    std::thread::spawn(move || unsafe {
        let com = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
        let mut file = [0u16; 1024];
        for (i, c) in default_name.encode_utf16().take(file.len() - 1).enumerate() {
            file[i] = c;
        }
        let desktop = wide(&known_folder(&FOLDERID_Desktop).unwrap_or_default().to_string_lossy());
        let mut ofn = OPENFILENAMEW {
            lStructSize: std::mem::size_of::<OPENFILENAMEW>() as u32,
            hwndOwner: HWND(owner as *mut _),
            lpstrFilter: PCWSTR(filter.as_ptr()),
            lpstrFile: PWSTR(file.as_mut_ptr()),
            nMaxFile: file.len() as u32,
            lpstrInitialDir: PCWSTR(desktop.as_ptr()),
            lpstrDefExt: w!("txt"),
            Flags: OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
            ..Default::default()
        };
        let chosen = GetSaveFileNameW(&mut ofn).as_bool();
        if com {
            CoUninitialize();
        }
        chosen.then(|| PathBuf::from(from_wide(&file)))
    })
    .join()
    .ok()
    .flatten()
}

/// The head of the report: the version, Windows, the account and the settings. The log goes after it.
pub fn summary(s: &AppState) -> String {
    let mut out = format!("SteamCenter {VERSION}\nWindows: {}\n\nAccount: ", windows_version());
    match (&s.account, &s.token) {
        (Some(a), Some(t)) => out.push_str(&format!("{} ({}), signed in until {} (unix), expired: {}\n", a.name, a.steamid, t.expires, t.expires_within(0))),
        (Some(a), None) => out.push_str(&format!("{} ({}), sign-in lost\n", a.name, a.steamid)),
        _ => out.push_str("not signed in\n"),
    }
    out.push_str(&format!("\nSettings: {}\n\nLog:\n", serde_json::to_string(&s.settings).unwrap_or_default()));
    out
}
