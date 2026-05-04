//! Editable cipher form — the flip-side of `cipher_detail`.
//!
//! Layout mirrors the Angular `add-edit-v2` component so field coverage and
//! grouping match the official clients. Same section primitives as
//! `cipher_detail` (via `field_helpers`) so view and edit modes share a
//! consistent visual rhythm.
//!
//! `CipherForm` holds two `CipherView`s: an untouched `original` (for cancel
//! and diffing) and a `modified` that form events mutate in place. Save hands
//! `modified` back to `ClientManager::save_cipher` which encrypts and
//! persists via the per-user SQLite repo.
//!
//! File layout:
//! - `state.rs`     — `CipherForm` + choice enums + constructors
//! - `message.rs`   — `CipherEditMessage`, `FormEvent`
//! - `update.rs`    — `CipherForm::update`
//! - `view.rs`      — `view()` entry + header/bottom-bar
//! - `selectors.rs` — folder/org/collections selectors + dropdown primitive
//! - `sections/`    — per-cipher-type section cards

mod message;
mod sections;
mod selectors;
mod state;
mod update;
mod view;

pub use message::{CipherEditMessage, FormEvent};
pub use sections::shared::NAME_INPUT_ID;
pub use state::{CipherForm, FolderOption};
pub use view::view;
