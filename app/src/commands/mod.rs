//! IPC commands the window calls: a thin layer that checks the input, calls the services and returns the answer.
//! Commands that talk to Steam are async: they run off the window's thread.

pub mod account;
pub mod app;
pub mod friends;
pub mod library;
pub mod profile;
pub mod security;
pub mod update;
pub mod wishlist;
