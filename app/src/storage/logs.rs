//! The log: `steamcenter.log` in the app's log folder (rotated at 5 MB, three old files kept), the latest lines in memory
//! for the "Diagnostics" page, and a "log" event to the main window. Tokens and cookies never get here.
//!
//! One entry is one line: `2026-11-15 14:02:11 WARNING [library] message`.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

const KEEP_LINES: usize = 1500;
const MAX_BYTES: u64 = 5 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub ts: String,
    pub level: &'static str,
    pub module: String,
    pub message: String,
}

static LINES: Mutex<VecDeque<LogLine>> = Mutex::new(VecDeque::new());
/// The open log file and its size: a line costs one write.
static FILE: Mutex<Option<(File, u64)>> = Mutex::new(None);
static APP: OnceLock<AppHandle> = OnceLock::new();

pub fn attach(app: AppHandle) {
    let _ = APP.set(app);
}

pub fn recent() -> Vec<LogLine> {
    LINES.lock().unwrap().iter().cloned().collect()
}

fn level_name(level: log::Level) -> &'static str {
    match level {
        log::Level::Error => "ERROR",
        log::Level::Warn => "WARNING",
        _ => "INFO",
    }
}

/// The module that wrote the line: the last part of its path (or the explicit target); the crate root is "main".
fn module_name(target: &str) -> String {
    if target == env!("CARGO_CRATE_NAME") {
        return "main".to_string();
    }
    target.rsplit("::").next().unwrap_or(target).to_string()
}

/// Text from outside (a server's answer, a tool's output) may span lines; the log keeps one entry per line.
fn one_line(text: String) -> String {
    if text.contains(['\r', '\n']) {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    } else {
        text
    }
}

fn write_file(text: &str) {
    let path = crate::storage::store::log_dir().join("steamcenter.log");
    let mut f = FILE.lock().unwrap();
    if f.as_ref().is_some_and(|(_, size)| *size > MAX_BYTES) {
        *f = None;
        let _ = std::fs::rename(path.with_extension("2.log"), path.with_extension("3.log"));
        let _ = std::fs::rename(path.with_extension("1.log"), path.with_extension("2.log"));
        let _ = std::fs::rename(&path, path.with_extension("1.log"));
    }
    if f.is_none() {
        *f = OpenOptions::new().create(true).append(true).open(&path).ok().map(|file| {
            let size = file.metadata().map_or(0, |m| m.len());
            (file, size)
        });
    }
    if let Some((file, size)) = f.as_mut() {
        if file.write_all(text.as_bytes()).is_ok() {
            *size += text.len() as u64;
        }
    }
}

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }

    fn log(&self, r: &log::Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let now = chrono::Local::now();
        let line = LogLine {
            ts: now.format("%H:%M:%S").to_string(),
            level: level_name(r.level()),
            module: module_name(r.target()),
            message: one_line(r.args().to_string()),
        };
        write_file(&format!("{} {:<7} [{}] {}\r\n", now.format("%Y-%m-%d %H:%M:%S"), line.level, line.module, line.message));
        {
            let mut l = LINES.lock().unwrap();
            l.push_back(line.clone());
            while l.len() > KEEP_LINES {
                l.pop_front();
            }
        }
        if let Some(app) = APP.get() {
            let _ = app.emit_to("main", "log", &line);
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    let _ = std::fs::create_dir_all(crate::storage::store::log_dir());
    if log::set_logger(&Logger).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_one_line_with_a_module() {
        assert_eq!(module_name(concat!(env!("CARGO_CRATE_NAME"), "::commands::library")), "library");
        assert_eq!(module_name(env!("CARGO_CRATE_NAME")), "main");
        assert_eq!(module_name("net"), "net");
        assert_eq!(one_line("first line\r\n  second\nthird".into()), "first line second third");
        assert_eq!(one_line("kept  as is".into()), "kept  as is");
    }
}
