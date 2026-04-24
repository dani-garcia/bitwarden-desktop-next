//! SSH-key read-only section. Private key uses `reveal_field` (eye toggle +
//! copy); public key and fingerprint use the generic truncated-value + copy
//! button pattern from `sections::shared::field_with_action`.

use bitwarden_vault::SshKeyView;
use iced::{Element, Fill, widget::column};

use crate::{
    components::{icons, inputs::reveal_field},
    fl,
    theme::{AppColors, AppTheme},
    views::vault::widgets::{
        cipher_detail::CipherDetailMessage,
        field_helpers::{card_with_margin, styled_card},
    },
};

use super::shared::field_with_action;

pub(in super::super) fn ssh_key_card<'a>(
    key: &'a SshKeyView,
    colors: &'a AppColors,
) -> Element<'a, CipherDetailMessage, AppTheme> {
    let fields = vec![
        reveal_field(
            fl!("detail-field-private-key"),
            &key.private_key,
            Some(CipherDetailMessage::CopySshPrivateKey),
            colors,
        ),
        field_with_action(
            fl!("detail-field-public-key"),
            key.public_key.as_str(),
            &[icons::BWI_COPY],
            &[CipherDetailMessage::CopySshPublicKey],
            colors,
        ),
        field_with_action(
            fl!("detail-field-fingerprint"),
            key.fingerprint.as_str(),
            &[icons::BWI_COPY],
            &[CipherDetailMessage::CopySshFingerprint],
            colors,
        ),
    ];
    card_with_margin(styled_card(column(fields).spacing(12).width(Fill).into()))
}
