//! Event handlers for the top-level `Message` enum, split by concern.
//!
//! Each submodule adds `impl App` methods for one slice of the message tree.
//! Split purely for readability — all methods remain on [`App`][super::App]
//! and call each other freely via `self.`.

mod login;
mod platform;
mod settings;
mod vault;
