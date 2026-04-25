//! Login-type section cards: credentials (username/password/TOTP + passkeys)
//! and autofill options (URIs).

use iced::{
    Alignment, Element, Fill,
    widget::{column, container, row, text},
};

use crate::{
    components::{
        buttons, icons,
        inputs::{reveal_text_field, text_field},
    },
    fl,
    theme::{AppColors, AppTheme},
    views::vault::widgets::{
        cipher_edit::{CipherEditMessage, CipherForm},
        field_helpers::{card_with_margin, field_readonly, format_passkey_date, styled_card},
    },
};

pub(in super::super) fn login_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let login = form.modified.login.as_ref().expect("ensure_sub_structs");

    let mut rows: Vec<Element<'a, CipherEditMessage, AppTheme>> = vec![
        text_field(
            fl!("form-username"),
            login.username.as_deref().unwrap_or(""),
            CipherEditMessage::UsernameChanged,
            None,
            form.saving,
            colors,
        ),
        reveal_text_field(
            fl!("form-password"),
            login.password.as_deref().unwrap_or(""),
            CipherEditMessage::PasswordChanged,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-totp"),
            login.totp.as_deref().unwrap_or(""),
            CipherEditMessage::TotpChanged,
            None,
            form.saving,
            colors,
        ),
    ];

    // Passkey rows — read-only display of creation date plus a delete
    if let Some(creds) = login.fido2_credentials.as_deref() {
        for (idx, cred) in creds.iter().enumerate() {
            let info = field_readonly(
                fl!("detail-field-passkey"),
                fl!(
                    "detail-field-passkey-created",
                    date = format_passkey_date(cred.creation_date)
                ),
                colors,
            );
            let remove_btn = buttons::ghost_icon(
                icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
                colors.item_hover,
            )
            .on_press(CipherEditMessage::PasskeyRemoved(idx))
            .padding([6, 6]);

            rows.push(
                row![container(info).width(Fill), remove_btn]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }
    }

    card_with_margin(styled_card(column(rows).spacing(16).into()))
}

pub(in super::super) fn autofill_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherEditMessage, AppTheme>> = Vec::new();

    let uris = form
        .modified
        .login
        .as_ref()
        .and_then(|l| l.uris.as_deref())
        .unwrap_or(&[]);

    if uris.is_empty() {
        rows.push(
            text(fl!("form-uri-empty"))
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    } else {
        for (idx, uri) in uris.iter().enumerate() {
            let value = uri.uri.as_deref().unwrap_or("");
            let input = text_field(
                fl!("form-uri"),
                value,
                move |s| CipherEditMessage::UriChanged(idx, s),
                None,
                form.saving,
                colors,
            );
            let remove_btn = buttons::ghost_icon(
                icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
                colors.item_hover,
            )
            .on_press(CipherEditMessage::UriRemoved(idx))
            .padding([6, 6]);

            rows.push(
                row![container(input).width(Fill), remove_btn,]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }
    }

    let add_btn = buttons::secondary(
        row![
            icons::PLUS.render(14.0, colors.accent),
            text(fl!("form-add-website")).size(14),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(CipherEditMessage::UriAdded)
    .padding([6, 12]);
    rows.push(add_btn.into());

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}
