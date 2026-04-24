//! Read-only cipher detail pane — the flip side of
//! [`super::cipher_edit`]. Each cipher type's rendering lives in
//! `sections/{type}.rs`, mirroring `cipher_edit/sections/{type}.rs` so
//! finding the renderer is the same motion on both sides.
//!
//! File layout:
//! - `message.rs`   — `CipherDetailMessage`
//! - `view.rs`      — top-level `view()` + header + bottom bar
//! - `sections/`    — per-cipher-type section cards (+ `shared.rs` for
//!   cross-type primitives like `field_with_action`)

mod message;
mod sections;
mod view;

pub use message::CipherDetailMessage;
pub(crate) use sections::login::login_uri_at;
pub use view::view;
