//! Per-cipher-type read-only section cards. Mirrors
//! [`super::super::cipher_edit::sections`] — for each cipher type, the
//! read-only rendering lives in `{type}.rs` here and the editable
//! rendering lives in the parallel file under `cipher_edit/sections`.

pub(super) mod bank_account;
pub(super) mod card;
pub(super) mod drivers_license;
pub(super) mod identity;
pub(super) mod login;
pub(super) mod passport;
pub(super) mod shared;
pub(super) mod ssh_key;
