//! Data in `%APPDATA%\SteamCenter\`: the account, settings, the window's place and backups of the private-games list.
//! The access token is stored only encrypted with DPAPI (bound to the Windows user).

use std::collections::BTreeSet;
use std::path::PathBuf;

use base64::Engine;
use serde::{Deserialize, Serialize};

pub fn data_dir() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".")).join("SteamCenter")
}

pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// The browser data of the app's windows (WebView2): the Steam sign-in keeps its cookies here.
pub fn webview_dir() -> PathBuf {
    data_dir().join("webview")
}

/// A downloaded update and the replaced exe.
pub fn update_dir() -> PathBuf {
    data_dir().join("update")
}

fn backup_dir() -> PathBuf {
    data_dir().join("backups")
}

/// A DPAPI-encrypted secret. In JSON - base64; `Debug` does not print the content.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Sealed(String);

impl std::fmt::Debug for Sealed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_empty() { "Sealed(empty)" } else { "Sealed(***)" })
    }
}

impl Sealed {
    pub fn seal(plain: &str) -> Result<Self, String> {
        if plain.is_empty() {
            return Ok(Self::default());
        }
        let blob = dpapi::protect(plain.as_bytes())?;
        Ok(Self(base64::engine::general_purpose::STANDARD.encode(blob)))
    }

    pub fn open(&self) -> Result<String, String> {
        if self.0.is_empty() {
            return Ok(String::new());
        }
        let blob = base64::engine::general_purpose::STANDARD.decode(&self.0).map_err(|e| e.to_string())?;
        String::from_utf8(dpapi::unprotect(&blob)?).map_err(|e| e.to_string())
    }
}

/// The signed-in Steam account: its name and avatar are kept for the window, the token - sealed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub steamid: String,
    pub name: String,
    pub avatar: String,
    pub token: Sealed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    /// The close button hides the window to the tray instead of quitting.
    pub close_to_tray: bool,
    /// "Check for updates" by itself at startup and install a newer version. On by default: people forget to update.
    pub auto_update: bool,
    /// Card idling leaves out games with Valve Anti-Cheat. On by default: to be on the safe side.
    pub skip_vac: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { language: crate::platform::system::ui_language().into(), close_to_tray: false, auto_update: true, skip_vac: true }
    }
}

/// A file that can't be read as JSON is kept as `<name>.bad` rather than silently replaced by the next save.
fn read_json<T: for<'de> Deserialize<'de> + Default>(path: PathBuf) -> T {
    let Ok(text) = std::fs::read_to_string(&path) else { return T::default() };
    serde_json::from_str(&text).unwrap_or_else(|_| {
        let _ = std::fs::rename(&path, path.with_extension("bad"));
        T::default()
    })
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(value).unwrap()).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())
}

pub fn load_account() -> Option<Account> {
    read_json(data_dir().join("account.json"))
}

pub fn save_account(a: Option<&Account>) -> Result<(), String> {
    let path = data_dir().join("account.json");
    match a {
        Some(a) => write_json(path, a),
        None => match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
    }
}

pub fn load_settings() -> Settings {
    read_json(data_dir().join("settings.json"))
}

pub fn save_settings(s: &Settings) -> Result<(), String> {
    write_json(data_dir().join("settings.json"), s)
}

/// The main window's normal place between runs (outer rectangle in pixels) and whether it was maximized.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

pub fn load_window() -> Option<WindowGeometry> {
    read_json(data_dir().join("window.json"))
}

pub fn save_window(g: &WindowGeometry) -> Result<(), String> {
    write_json(data_dir().join("window.json"), g)
}

/// The private-games list as it was before a change: made before every change, to undo it.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Backup {
    steamid: String,
    created: i64,
    appids: BTreeSet<u32>,
}

#[derive(Debug, Serialize)]
pub struct BackupInfo {
    pub name: String,
    pub created: i64,
    pub count: usize,
}

/// How many backups are kept (the oldest go first).
const KEEP_BACKUPS: usize = 30;

pub fn save_backup(steamid: &str, appids: &BTreeSet<u32>) -> Result<(), String> {
    let now = chrono::Local::now();
    let name = format!("private-{}.json", now.format("%Y-%m-%d_%H-%M-%S"));
    write_json(backup_dir().join(name), &Backup { steamid: steamid.into(), created: now.timestamp(), appids: appids.clone() })?;
    let mut names = backup_names();
    names.sort();
    for old in names.iter().take(names.len().saturating_sub(KEEP_BACKUPS)) {
        let _ = std::fs::remove_file(backup_dir().join(old));
    }
    Ok(())
}

fn backup_names() -> Vec<String> {
    let Ok(dir) = std::fs::read_dir(backup_dir()) else { return Vec::new() };
    dir.filter_map(|e| e.ok()?.file_name().into_string().ok()).filter(|n| valid_backup_name(n)).collect()
}

/// Only our own file names: the window sends a name back, and it must not lead out of the folder.
fn valid_backup_name(name: &str) -> bool {
    name.starts_with("private-") && name.ends_with(".json") && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b)) && !name.contains("..")
}

/// The backups of this account, newest first.
pub fn list_backups(steamid: &str) -> Vec<BackupInfo> {
    let mut list: Vec<BackupInfo> = backup_names()
        .into_iter()
        .filter_map(|name| {
            let b: Backup = serde_json::from_str(&std::fs::read_to_string(backup_dir().join(&name)).ok()?).ok()?;
            (b.steamid == steamid).then_some(BackupInfo { name, created: b.created, count: b.appids.len() })
        })
        .collect();
    list.sort_by(|a, b| b.name.cmp(&a.name));
    list
}

pub fn load_backup(steamid: &str, name: &str) -> Option<BTreeSet<u32>> {
    if !valid_backup_name(name) {
        return None;
    }
    let b: Backup = serde_json::from_str(&std::fs::read_to_string(backup_dir().join(name)).ok()?).ok()?;
    (b.steamid == steamid).then_some(b.appids)
}

mod dpapi {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    /// Run a DPAPI call on `data` and take its output buffer.
    fn call(data: &[u8], f: impl FnOnce(&CRYPT_INTEGER_BLOB, &mut CRYPT_INTEGER_BLOB) -> windows::core::Result<()>) -> Result<Vec<u8>, String> {
        let inb = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB::default();
        f(&inb, &mut out).map_err(|e| format!("DPAPI: {e}"))?;
        unsafe {
            let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
            let _ = LocalFree(HLOCAL(out.pbData as _));
            Ok(v)
        }
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        call(data, |i, o| unsafe { CryptProtectData(i, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, o) })
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        call(data, |i, o| unsafe { CryptUnprotectData(i, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, o) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpapi_roundtrip_and_no_plaintext() {
        let s = Sealed::seal("eyJ0eXAiOiJKV1Qi.secret-token").unwrap();
        assert_eq!(s.open().unwrap(), "eyJ0eXAiOiJKV1Qi.secret-token");
        let json = serde_json::to_string(&s).unwrap();
        assert!(!json.contains("secret-token"));
        assert_eq!(format!("{s:?}"), "Sealed(***)");
        assert_eq!(Sealed::seal("").unwrap().open().unwrap(), "");
    }

    #[test]
    fn settings_defaults_fill_missing_fields() {
        let s: Settings = serde_json::from_str(r#"{"language":"en"}"#).unwrap();
        assert_eq!(s.language, "en");
        assert!(s.auto_update);
        assert!(s.skip_vac);
        assert!(!s.close_to_tray);
    }

    #[test]
    fn backup_names_cannot_leave_the_folder() {
        assert!(valid_backup_name("private-2026-11-16_10-00-00.json"));
        assert!(!valid_backup_name("private-..\\..\\account.json"));
        assert!(!valid_backup_name("private-../x.json"));
        assert!(!valid_backup_name("account.json"));
    }
}
