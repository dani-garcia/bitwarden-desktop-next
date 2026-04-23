//! SSH-key section card. Read-only today — key regeneration / import is
//! deferred.

use bitwarden_vault::SshKeyView;
use iced::{Element, widget::column};

use crate::{
    fl,
    theme::{AppColors, AppTheme},
};

use super::super::super::field_helpers::{card_with_margin, field_readonly, styled_card};
use super::super::{message::CipherFormMessage, state::CipherForm};

pub(in super::super) fn ssh_key_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let default_key = SshKeyView {
        private_key: String::new(),
        public_key: String::new(),
        fingerprint: String::new(),
    };
    let k = form.modified.ssh_key.as_ref().unwrap_or(&default_key);
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        field_readonly(fl!("form-ssh-public-key"), k.public_key.clone(), colors),
        field_readonly(fl!("form-ssh-private-key"), k.private_key.clone(), colors),
        field_readonly(fl!("form-ssh-fingerprint"), k.fingerprint.clone(), colors),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}
