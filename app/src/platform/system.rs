//! Facts about this Windows and this process, for the log and the support report; opening links and waiting for a
//! process.

use std::time::Duration;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ};
use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes, OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use super::win32::{from_wide, wide};

/// Milliseconds since Windows created this process: includes loading the exe (and an antivirus check of it) before
/// `main`. For the startup lines in the log.
pub fn since_start_ms() -> u64 {
    let ticks = |f: FILETIME| (u64::from(f.dwHighDateTime) << 32) | u64::from(f.dwLowDateTime);
    let (mut created, mut exited, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default(), FILETIME::default());
    if unsafe { GetProcessTimes(GetCurrentProcess(), &mut created, &mut exited, &mut kernel, &mut user) }.is_err() {
        return 0;
    }
    // FILETIME counts 100 ns.
    ticks(unsafe { GetSystemTimeAsFileTime() }).saturating_sub(ticks(created)) / 10_000
}

/// Windows version from the registry (without running `cmd /c ver`, so no console window flashes).
pub fn windows_version() -> String {
    let key = w!(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion");
    let text = |name: PCWSTR| -> String {
        let mut buf = [0u16; 128];
        let mut size = (buf.len() * 2) as u32;
        let _ = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, key, name, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size)) };
        from_wide(&buf)
    };
    let mut ubr = 0u32;
    let mut size = 4u32;
    let _ = unsafe { RegGetValueW(HKEY_LOCAL_MACHINE, key, w!("UBR"), RRF_RT_REG_DWORD, None, Some((&mut ubr as *mut u32).cast()), Some(&mut size)) };
    let build: u32 = text(w!("CurrentBuild")).parse().unwrap_or(0);
    let name = if build >= 22000 { "Windows 11" } else { "Windows 10" };
    format!("{name} {} (build {build}.{ubr})", text(w!("DisplayVersion")))
}

/// The interface language for the first start: Russian for Windows in Russian (and Ukrainian, Belarusian, Kazakh,
/// where Russian is read more often than English), English otherwise.
pub fn ui_language() -> &'static str {
    match unsafe { GetUserDefaultUILanguage() } & 0x3ff {
        0x19 | 0x22 | 0x23 | 0x3f => "ru",
        _ => "en",
    }
}

/// Open a link in the default browser.
pub fn open_url(url: &str) -> Result<(), String> {
    let u = wide(url);
    let r = unsafe { ShellExecuteW(None, w!("open"), PCWSTR(u.as_ptr()), None, None, SW_SHOWNORMAL) };
    // ShellExecute returns a value above 32 on success.
    if r.0 as isize > 32 {
        Ok(())
    } else {
        Err(format!("ShellExecute: {}", r.0 as isize))
    }
}

/// Wait for a process to end (the previous copy after an update), at most `timeout`.
pub fn wait_exit(pid: u32, timeout: Duration) {
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            WaitForSingleObject(h, timeout.as_millis() as u32);
            let _ = CloseHandle(h);
        }
    }
}
