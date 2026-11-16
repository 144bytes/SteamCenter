//! Long operations that change the account (mass privacy changes, wishlist and friends cleanup): one at a time, with progress
//! for the window. Quitting and updating wait for them.

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::error::{msg, Msg};
use crate::state::{notify, Shared};

/// The running operation; it ends when this is dropped, whatever way the command returns.
pub struct Job {
    app: AppHandle,
}

pub fn start(app: &AppHandle, name: &'static str) -> Result<Job, Msg> {
    let state = app.state::<Shared>();
    let mut s = state.lock().unwrap();
    if s.job.is_some() {
        return Err(msg("err.busy"));
    }
    s.job = Some(name);
    drop(s);
    notify(app);
    Ok(Job { app: app.clone() })
}

pub fn running(app: &AppHandle) -> bool {
    app.state::<Shared>().lock().unwrap().job.is_some()
}

impl Job {
    /// `done` of `total` items.
    pub fn progress(&self, done: usize, total: usize) {
        let _ = self.app.emit_to("main", "progress", json!({ "done": done, "total": total }));
    }
}

impl Drop for Job {
    fn drop(&mut self) {
        self.app.state::<Shared>().lock().unwrap().job = None;
        let _ = self.app.emit_to("main", "progress", ());
        notify(&self.app);
    }
}
