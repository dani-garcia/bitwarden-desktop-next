//! Per-cipher-type section cards. `view.rs` picks cards by matching on
//! `CipherType` and splices the chosen set between the universal
//! `item_details` header and the universal `additional_options` /
//! `custom_fields` footers.

pub(super) mod bank_account;
pub(super) mod card;
pub(super) mod drivers_license;
pub(super) mod identity;
pub(super) mod login;
pub(super) mod passport;
pub(super) mod shared;
pub(super) mod ssh_key;
