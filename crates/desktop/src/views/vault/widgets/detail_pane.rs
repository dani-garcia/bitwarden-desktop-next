use bitwarden_vault::{
    CardView, CipherType, CipherView, FieldType, FieldView, IdentityView, LoginView, SshKeyView,
};
use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, row, scrollable, text},
};

use super::field_helpers::{
    card_with_margin, field_readonly, format_passkey_date, section_label, styled_card,
};
use crate::{
    components::{self, buttons, buttons::icon_button, icons, inputs::reveal_field},
    fl,
    theme::{AppColors, AppTheme},
};

#[derive(Debug, Clone)]
pub enum DetailPaneMessage {
    Close,
    CopyUsername,
    CopyPassword,
    /// Copy the URI at the given index in `login.uris`. A login may have
    /// multiple URIs; the autofill section renders one button row per URI.
    CopyUrl(usize),
    CopyTotp,
    /// Open the URI at the given index in `login.uris`. See `CopyUrl`.
    OpenUrl(usize),
    /// Copy the value of the custom field at the given index in
    /// `cipher.fields`. Only wired for hidden-type fields today; plain
    /// text fields can still be selected and copied manually.
    CopyCustomField(usize),
    Edit,
    /// Trash icon pressed — opens the confirm modal (handled at the vault
    /// view layer). Does not actually delete by itself.
    Delete,
}

pub fn view<'a>(
    item: &'a CipherView,
    colors: &'a AppColors,
    top_radius: f32,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let header = header_row(item, colors);

    let mut sections: Vec<Element<'a, DetailPaneMessage, AppTheme>> = vec![
        section_label(fl!("detail-section-item-details"), colors),
        item_details_card(item, colors),
    ];

    match item.r#type {
        CipherType::Login => {
            if let Some(login) = item.login.as_ref() {
                sections.push(section_label(
                    fl!("detail-section-login-credentials"),
                    colors,
                ));
                sections.push(login_card(login, colors));

                let uris = collect_login_uris(login);
                if !uris.is_empty() {
                    sections.push(section_label(
                        fl!("detail-section-autofill-options"),
                        colors,
                    ));
                    sections.push(autofill_card(&uris, colors));
                }
            }
        }
        CipherType::Card => {
            if let Some(card) = item.card.as_ref() {
                sections.push(section_label(fl!("detail-section-card-details"), colors));
                sections.push(card_details_card(card, colors));
            }
        }
        CipherType::Identity => {
            if let Some(identity) = item.identity.as_ref() {
                sections.push(section_label(
                    fl!("detail-section-personal-details"),
                    colors,
                ));
                sections.push(identity_card(identity, colors));
            }
        }
        CipherType::SecureNote => {
            if let Some(notes) = item.notes.as_deref() {
                sections.push(section_label(fl!("detail-section-note"), colors));
                sections.push(card_with_margin(styled_card(
                    text(notes).size(14).color(colors.text_primary).into(),
                )));
            }
        }
        CipherType::SshKey => {
            if let Some(key) = item.ssh_key.as_ref() {
                sections.push(section_label(fl!("detail-section-ssh-key"), colors));
                sections.push(ssh_key_card(key, colors));
            }
        }
    }

    if let Some(fields) = item.fields.as_deref().filter(|f| !f.is_empty()) {
        sections.push(section_label(fl!("detail-section-custom-fields"), colors));
        sections.push(custom_fields_card(fields, colors));
    }

    let body = scrollable(column(sections).spacing(4).padding([12, 20])).height(Fill);
    let bottom_bar = bottom_bar(colors);

    container(column![header, body, bottom_bar].spacing(0).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    iced::Border::default()
                        .rounded(iced::border::top_left(top_radius).top_right(top_radius)),
                )
        })
        .into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn header_row<'a>(
    item: &'a CipherView,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let category_label = match item.r#type {
        CipherType::Login => fl!("detail-header-login"),
        CipherType::Card => fl!("detail-header-card"),
        CipherType::Identity => fl!("detail-header-identity"),
        CipherType::SecureNote => fl!("detail-header-note"),
        CipherType::SshKey => fl!("detail-header-ssh-key"),
    };

    let title = text(category_label).size(18).color(colors.text_primary);

    let close_btn = buttons::ghost_icon(
        icons::BWI_CLOSE.render(32.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(DetailPaneMessage::Close)
    .padding([1, 1]);

    // No background fill on the header — it inherits the parent container's
    // `card_bg`. A separate background here would mask the parent's rounded
    // top corners (iced doesn't clip children to parent border radius).
    let header =
        container(row![title, Space::new().width(Fill), close_btn].align_y(Alignment::Center))
            .padding([8, 20]);

    column![header, components::separator_h()].spacing(0).into()
}

// ---------------------------------------------------------------------------
// Item details (universal)
// ---------------------------------------------------------------------------

fn item_details_card<'a>(
    item: &'a CipherView,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> =
        vec![field_readonly(fl!("detail-field-name"), &item.name, colors)];
    if let Some(notes) = item.notes.as_deref()
        && !matches!(item.r#type, CipherType::SecureNote)
    {
        fields.push(field_readonly(fl!("detail-field-notes"), notes, colors));
    }
    card_with_margin(styled_card(column(fields).spacing(12).width(Fill).into()))
}

// ---------------------------------------------------------------------------
// Login card
// ---------------------------------------------------------------------------

fn login_card<'a>(
    login: &'a LoginView,
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> = Vec::new();

    if let Some(username) = login.username.as_deref() {
        fields.push(field_with_action(
            fl!("detail-field-username"),
            username,
            &[icons::BWI_COPY],
            &[DetailPaneMessage::CopyUsername],
            colors,
        ));
    }

    if let Some(password) = login.password.as_deref() {
        fields.push(reveal_field(
            fl!("detail-field-password"),
            password,
            Some(DetailPaneMessage::CopyPassword),
            colors,
        ));
    }

    if let Some(totp) = login.totp.as_deref().filter(|s| !s.is_empty()) {
        fields.push(components::totp::view(
            totp,
            DetailPaneMessage::CopyTotp,
            colors,
        ));
    }

    // `fido2_credentials` holds the still-encrypted `Fido2Credential` (the
    // SDK keeps this field opaque on the view — see the upstream TODO on
    // `LoginView`). The only plaintext field is `creation_date`, so that's
    // all we surface here.
    if let Some(creds) = login.fido2_credentials.as_deref() {
        for cred in creds {
            fields.push(field_readonly(
                fl!("detail-field-passkey"),
                fl!(
                    "detail-field-passkey-created",
                    date = format_passkey_date(cred.creation_date)
                ),
                colors,
            ));
        }
    }

    if fields.is_empty() {
        fields.push(
            text(fl!("detail-empty-credentials"))
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }

    card_with_margin(styled_card(column(fields).spacing(16).width(Fill).into()))
}

/// Non-empty URIs on a login, paired with their original index in
/// `login.uris`. The index is the stable key used by the `CopyUrl` /
/// `OpenUrl` messages, so we must preserve position — callers should
/// not sort or re-index this.
pub(crate) fn collect_login_uris(login: &LoginView) -> Vec<(usize, &str)> {
    login
        .uris
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .enumerate()
        .filter_map(|(i, u)| u.uri.as_deref().map(|s| (i, s)))
        .collect()
}

/// Convenience used by handlers that only care about a single URI by its
/// position in `login.uris`.
pub(crate) fn login_uri_at(login: &LoginView, index: usize) -> Option<&str> {
    login
        .uris
        .as_deref()?
        .get(index)
        .and_then(|u| u.uri.as_deref())
}

// ---------------------------------------------------------------------------
// Card card
// ---------------------------------------------------------------------------

fn card_details_card<'a>(
    card: &'a CardView,
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> = Vec::new();
    push_optional_field(
        &mut fields,
        fl!("detail-field-cardholder-name"),
        card.cardholder_name.as_deref(),
        colors,
    );
    push_optional_field(
        &mut fields,
        fl!("detail-field-brand"),
        card.brand.as_deref(),
        colors,
    );
    push_optional_field(
        &mut fields,
        fl!("detail-field-number"),
        card.number.as_deref(),
        colors,
    );

    let expiration = match (card.exp_month.as_deref(), card.exp_year.as_deref()) {
        (Some(m), Some(y)) => Some(format!("{m}/{y}")),
        (Some(m), None) => Some(m.to_string()),
        (None, Some(y)) => Some(y.to_string()),
        (None, None) => None,
    };
    if let Some(exp) = expiration {
        fields.push(field_readonly(fl!("detail-field-expiration"), exp, colors));
    }

    if let Some(code) = card.code.as_deref() {
        fields.push(reveal_field(
            fl!("detail-field-security-code"),
            code,
            None,
            colors,
        ));
    }

    if fields.is_empty() {
        fields.push(
            text(fl!("detail-empty-card"))
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }
    card_with_margin(styled_card(column(fields).spacing(12).width(Fill).into()))
}

// ---------------------------------------------------------------------------
// Identity card
// ---------------------------------------------------------------------------

fn identity_card<'a>(
    identity: &'a IdentityView,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> = Vec::new();

    let full_name = [
        identity.title.as_deref(),
        identity.first_name.as_deref(),
        identity.middle_name.as_deref(),
        identity.last_name.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ");
    if !full_name.is_empty() {
        fields.push(field_readonly(fl!("detail-field-name"), full_name, colors));
    }

    push_optional_field(
        &mut fields,
        fl!("detail-field-email"),
        identity.email.as_deref(),
        colors,
    );
    push_optional_field(
        &mut fields,
        fl!("detail-field-phone"),
        identity.phone.as_deref(),
        colors,
    );
    push_optional_field(
        &mut fields,
        fl!("detail-field-company"),
        identity.company.as_deref(),
        colors,
    );

    let address_lines = [
        identity.address1.as_deref(),
        identity.address2.as_deref(),
        identity.address3.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    if !address_lines.is_empty() {
        fields.push(field_readonly(
            fl!("detail-field-address"),
            address_lines,
            colors,
        ));
    }

    let locality = [
        identity.city.as_deref(),
        identity.state.as_deref(),
        identity.postal_code.as_deref(),
        identity.country.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(", ");
    if !locality.is_empty() {
        fields.push(field_readonly(
            fl!("detail-field-city-region"),
            locality,
            colors,
        ));
    }

    if fields.is_empty() {
        fields.push(
            text(fl!("detail-empty-identity"))
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }
    card_with_margin(styled_card(column(fields).spacing(12).width(Fill).into()))
}

// ---------------------------------------------------------------------------
// SSH key card
// ---------------------------------------------------------------------------

fn ssh_key_card<'a>(
    key: &'a SshKeyView,
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let fields = vec![
        field_readonly(fl!("detail-field-public-key"), &key.public_key, colors),
        reveal_field(
            fl!("detail-field-private-key"),
            &key.private_key,
            None,
            colors,
        ),
        field_readonly(fl!("detail-field-fingerprint"), &key.fingerprint, colors),
    ];
    card_with_margin(styled_card(column(fields).spacing(12).width(Fill).into()))
}

// ---------------------------------------------------------------------------
// Custom fields
// ---------------------------------------------------------------------------

fn custom_fields_card<'a>(
    fields: &'a [FieldView],
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let rows: Vec<Element<'a, DetailPaneMessage, AppTheme>> = fields
        .iter()
        .enumerate()
        .map(|(idx, f)| custom_field(idx, f, colors))
        .collect();
    card_with_margin(styled_card(column(rows).spacing(12).width(Fill).into()))
}

fn custom_field<'a>(
    idx: usize,
    field: &'a FieldView,
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let label = field.name.as_deref().unwrap_or("").to_string();
    let raw_value = field.value.as_deref().unwrap_or("");
    match field.r#type {
        FieldType::Hidden => reveal_field(
            label,
            raw_value,
            Some(DetailPaneMessage::CopyCustomField(idx)),
            colors,
        ),
        FieldType::Boolean => {
            let v = if matches!(raw_value, "true") {
                fl!("detail-field-boolean-true")
            } else {
                fl!("detail-field-boolean-false")
            };
            field_with_action(
                label,
                v,
                &[icons::BWI_COPY],
                &[DetailPaneMessage::CopyCustomField(idx)],
                colors,
            )
        }
        FieldType::Text => field_with_action(
            label,
            raw_value,
            &[icons::BWI_COPY],
            &[DetailPaneMessage::CopyCustomField(idx)],
            colors,
        ),
        // Linked fields aren't usefully renderable without resolving the
        // target property name, matching the stance in `cipher_form`.
        FieldType::Linked => field_readonly(label, raw_value, colors),
    }
}

// ---------------------------------------------------------------------------
// Autofill card
// ---------------------------------------------------------------------------

fn autofill_card<'a>(
    uris: &[(usize, &'a str)],
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let rows: Vec<Element<'a, DetailPaneMessage, AppTheme>> = uris
        .iter()
        .copied()
        .map(|(idx, uri)| autofill_row(idx, uri, colors))
        .collect();
    card_with_margin(styled_card(column(rows).spacing(12).width(Fill).into()))
}

fn autofill_row<'a>(
    idx: usize,
    uri: &'a str,
    colors: &'a AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let label = text(fl!("detail-field-website"))
        .size(12)
        .color(colors.text_muted);
    // Same pattern as other detail fields: avoid wrapping so the buttons
    // stay aligned with the label row for long URIs.
    let value = text(uri)
        .size(14)
        .color(colors.text_primary)
        .wrapping(iced::widget::text::Wrapping::None)
        .ellipsis(iced::widget::text::Ellipsis::End);

    let copy_btn = icon_button(icons::BWI_COPY, DetailPaneMessage::CopyUrl(idx), colors);
    let open_btn = icon_button(
        icons::BWI_EXTERNAL_LINK,
        DetailPaneMessage::OpenUrl(idx),
        colors,
    );

    row![
        column![label, value].spacing(2).width(Fill),
        copy_btn,
        open_btn,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

// ---------------------------------------------------------------------------
// Bottom bar
// ---------------------------------------------------------------------------

fn bottom_bar<'a>(colors: &AppColors) -> Element<'a, DetailPaneMessage, AppTheme> {
    let edit_btn = buttons::primary(text(fl!("detail-edit-button")).size(14))
        .on_press(DetailPaneMessage::Edit)
        .padding([8, 20]);

    let delete_btn = buttons::ghost_icon(
        icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
        colors.item_hover,
    )
    .on_press(DetailPaneMessage::Delete)
    .padding([6, 6]);

    let bar =
        container(row![edit_btn, Space::new().width(Fill), delete_btn].align_y(Alignment::Center))
            .padding([8, 20])
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.background)
            });

    column![components::separator_h(), bar].spacing(0).into()
}

// ---------------------------------------------------------------------------
// Helpers (detail-pane-specific; shared primitives live in `field_helpers`)
// ---------------------------------------------------------------------------

fn push_optional_field<'a>(
    fields: &mut Vec<Element<'a, DetailPaneMessage, AppTheme>>,
    label: impl Into<String>,
    value: Option<&'a str>,
    colors: &AppColors,
) {
    if let Some(v) = value {
        fields.push(field_readonly(label, v, colors));
    }
}

fn field_with_action<'a>(
    label: impl Into<String>,
    value: impl iced::widget::text::IntoFragment<'a>,
    icons_list: &[icons::BwiIcon],
    msgs: &[DetailPaneMessage],
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    use iced::widget::text::{Ellipsis, Wrapping};

    let buttons: Vec<Element<'a, DetailPaneMessage, AppTheme>> = icons_list
        .iter()
        .zip(msgs.iter())
        .map(|(icon, msg)| icon_button(*icon, msg.clone(), colors))
        .collect();

    let buttons_row = row(buttons).spacing(2).align_y(Alignment::Center);

    row![
        column![
            text(label.into()).size(12).color(colors.text_muted),
            text(value)
                .size(14)
                .color(colors.text_primary)
                .wrapping(Wrapping::None)
                .ellipsis(Ellipsis::End),
        ]
        .spacing(2)
        .width(Fill),
        buttons_row,
    ]
    .spacing(4)
    .width(Fill)
    .align_y(Alignment::Center)
    .into()
}
