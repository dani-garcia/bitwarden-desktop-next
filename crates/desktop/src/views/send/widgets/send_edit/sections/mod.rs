//! Per-card sections of the send form. `view.rs` stitches the two cards
//! between the shared header + footer; each section owns its own cluster of
//! fields so adding / re-ordering inputs stays local. Common primitives
//! (the rounded heading + card shell) live in `shared`.

pub(super) mod additional;
pub(super) mod details;
pub(super) mod shared;
