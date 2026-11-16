//! "Running" a game for Steam without the game: the program starts itself as `SteamCenter.exe --idle <appid> <pid>`.
//! That copy tells Steam it is the game (the `SteamAppId` variable), connects to the running Steam client through
//! `steamclient64.dll` of the installed Steam - the way Steam Achievement Manager does - and stays until the main
//! program kills it or exits. Steam counts the play time and drops the cards, as for the game itself.
//!
//! No helper exe and no `steam_api64.dll`: the program stays one file.

use std::ffi::{c_char, c_void};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command};

use windows::core::{s, w, PCSTR, PCWSTR};
use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryExW, SetDllDirectoryW, LOAD_WITH_ALTERED_SEARCH_PATH};
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
use windows::Win32::System::Threading::{OpenProcess, WaitForSingleObject, CREATE_NO_WINDOW, INFINITE, PROCESS_SYNCHRONIZE};

use super::win32::{from_wide, wide};

pub const IDLE_ARG: &str = "--idle";

/// Why an idling copy ended at once (its exit code).
pub const EXIT_NO_STEAM: i32 = 2;
pub const EXIT_NOT_RUNNING: i32 = 3;

/// The installed Steam's folder (from the registry, as Steam writes it).
pub fn steam_dir() -> Option<PathBuf> {
    let mut buf = [0u16; 520];
    let mut size = (buf.len() * 2) as u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, w!(r"Software\Valve\Steam"), w!("SteamPath"), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size))
            .ok()
            .ok()?;
    }
    let dir = PathBuf::from(from_wide(&buf).replace('/', "\\"));
    dir.join("steamclient64.dll").exists().then_some(dir)
}

/// Start an idling copy for `appid`.
pub fn spawn(appid: u32) -> std::io::Result<Child> {
    Command::new(std::env::current_exe()?)
        .args([IDLE_ARG, &appid.to_string(), &std::process::id().to_string()])
        .creation_flags(CREATE_NO_WINDOW.0)
        .spawn()
}

/// ISteamClient's first methods (their places in its table never changed between versions).
type CreateInterface = unsafe extern "C" fn(*const c_char, *mut i32) -> *mut c_void;
type CreateSteamPipe = unsafe extern "system" fn(*mut c_void) -> i32;
type ReleaseSteamPipe = unsafe extern "system" fn(*mut c_void, i32) -> bool;
type ConnectToGlobalUser = unsafe extern "system" fn(*mut c_void, i32) -> i32;
type ReleaseUser = unsafe extern "system" fn(*mut c_void, i32, i32);

/// The idling copy itself: `args` - [exe, --idle, appid, parent pid]. Returns the exit code.
pub fn child(args: &[String]) -> i32 {
    let (Some(appid), Some(parent)) = (args.get(2).and_then(|a| a.parse::<u32>().ok()), args.get(3).and_then(|p| p.parse::<u32>().ok()))
    else {
        return 1;
    };
    std::env::set_var("SteamAppId", appid.to_string());
    std::env::set_var("SteamGameId", appid.to_string());
    let Some(dir) = steam_dir() else { return EXIT_NO_STEAM };
    unsafe {
        // steamclient64.dll loads its neighbours (tier0, vstdlib) from the Steam folder.
        let d = wide(&dir.to_string_lossy());
        let _ = SetDllDirectoryW(PCWSTR(d.as_ptr()));
        let dll = wide(&dir.join("steamclient64.dll").to_string_lossy());
        let Ok(lib) = LoadLibraryExW(PCWSTR(dll.as_ptr()), None, LOAD_WITH_ALTERED_SEARCH_PATH) else { return EXIT_NO_STEAM };
        let Some(create) = GetProcAddress(lib, s!("CreateInterface")) else { return EXIT_NO_STEAM };
        let create: CreateInterface = std::mem::transmute(create);
        let client = ["SteamClient021\0", "SteamClient020\0", "SteamClient019\0", "SteamClient018\0", "SteamClient017\0"]
            .iter()
            .map(|v| create(PCSTR(v.as_ptr()).0.cast(), std::ptr::null_mut()))
            .find(|c| !c.is_null());
        let Some(client) = client else { return EXIT_NO_STEAM };
        let table = *(client as *const *const usize);
        let method = |i: usize| *table.add(i);
        let create_pipe: CreateSteamPipe = std::mem::transmute(method(0));
        let release_pipe: ReleaseSteamPipe = std::mem::transmute(method(1));
        let connect: ConnectToGlobalUser = std::mem::transmute(method(2));
        let release_user: ReleaseUser = std::mem::transmute(method(4));
        let pipe = create_pipe(client);
        if pipe == 0 {
            return EXIT_NOT_RUNNING;
        }
        let user = connect(client, pipe);
        if user == 0 {
            release_pipe(client, pipe);
            return EXIT_NOT_RUNNING;
        }
        // "Playing" until the program kills this copy, or ends itself.
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, parent) {
            WaitForSingleObject(h, INFINITE);
            let _ = CloseHandle(h);
        }
        release_user(client, pipe, user);
        release_pipe(client, pipe);
    }
    0
}
