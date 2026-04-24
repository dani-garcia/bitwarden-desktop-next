//! SSH-key section card. Read-only today — key regeneration / import is
//! deferred.

use bitwarden_vault::SshKeyView;
use iced::{Element, widget::column};

use crate::{
    components::inputs::readonly_field_truncated,
    fl,
    theme::{AppColors, AppTheme},
    views::vault::widgets::{
        cipher_edit::{CipherForm, CipherEditMessage},
        field_helpers::{card_with_margin, styled_card},
    },
};

pub(in super::super) fn ssh_key_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let default_key = SshKeyView {
        private_key: String::new(),
        public_key: String::new(),
        fingerprint: String::new(),
    };
    let k = form.modified.ssh_key.as_ref().unwrap_or(&default_key);
    let rows: Vec<Element<'a, CipherEditMessage, AppTheme>> = vec![
        readonly_field_truncated(fl!("form-ssh-private-key"), k.private_key.clone(), colors),
        readonly_field_truncated(fl!("form-ssh-public-key"), k.public_key.clone(), colors),
        readonly_field_truncated(fl!("form-ssh-fingerprint"), k.fingerprint.clone(), colors),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}
