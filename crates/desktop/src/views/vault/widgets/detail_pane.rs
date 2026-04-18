use bitwarden_vault::{CardView, CipherType, CipherView, IdentityView, LoginView, SshKeyView};
use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, row, scrollable, text},
};

use super::field_helpers::{
    card_with_margin, field_readonly, icon_button, section_label, styled_card,
};
use crate::{
    components::{self, buttons, icons},
    theme::{AppColors, AppTheme},
};

#[derive(Debug, Clone)]
pub enum DetailPaneMessage {
    Close,
    CopyUsername,
    CopyPassword,
    CopyUrl,
    OpenUrl,
    TogglePasswordVisibility,
    Edit,
    Delete,
}

pub fn view<'a>(
    item: &'a CipherView,
    colors: &AppColors,
    top_radius: f32,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let header = header_row(item, colors);

    let mut sections: Vec<Element<'a, DetailPaneMessage, AppTheme>> = vec![
        section_label("Item details", colors),
        item_details_card(item, colors),
    ];

    match item.r#type {
        CipherType::Login => {
            if let Some(login) = item.login.as_ref() {
                sections.push(section_label("Login credentials", colors));
                sections.push(login_card(login, colors));

                if let Some(uri) = first_login_uri(login) {
                    sections.push(section_label("Autofill options", colors));
                    sections.push(autofill_card(uri, colors));
                }
            }
        }
        CipherType::Card => {
            if let Some(card) = item.card.as_ref() {
                sections.push(section_label("Card details", colors));
                sections.push(card_details_card(card, colors));
            }
        }
        CipherType::Identity => {
            if let Some(identity) = item.identity.as_ref() {
                sections.push(section_label("Personal details", colors));
                sections.push(identity_card(identity, colors));
            }
        }
        CipherType::SecureNote => {
            if let Some(notes) = item.notes.as_deref() {
                sections.push(section_label("Note", colors));
                sections.push(card_with_margin(styled_card(
                    text(notes).size(14).color(colors.text_primary).into(),
                )));
            }
        }
        CipherType::SshKey => {
            if let Some(key) = item.ssh_key.as_ref() {
                sections.push(section_label("SSH key", colors));
                sections.push(ssh_key_card(key, colors));
            }
        }
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
        CipherType::Login => "View login",
        CipherType::Card => "View card",
        CipherType::Identity => "View identity",
        CipherType::SecureNote => "View note",
        CipherType::SshKey => "View SSH key",
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
        vec![field_readonly("Name", &item.name, colors)];
    if let Some(notes) = item.notes.as_deref()
        && !matches!(item.r#type, CipherType::SecureNote)
    {
        fields.push(field_readonly("Notes", notes, colors));
    }
    card_with_margin(styled_card(column(fields).spacing(12).into()))
}

// ---------------------------------------------------------------------------
// Login card
// ---------------------------------------------------------------------------

fn login_card<'a>(
    login: &'a LoginView,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> = Vec::new();

    if let Some(username) = login.username.as_deref() {
        fields.push(field_with_action(
            "Username",
            username,
            &[icons::BWI_COPY],
            &[DetailPaneMessage::CopyUsername],
            colors,
        ));
    }

    if login.password.is_some() {
        fields.push(field_with_action(
            "Password",
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}",
            &[icons::BWI_EYE, icons::BWI_COPY],
            &[
                DetailPaneMessage::TogglePasswordVisibility,
                DetailPaneMessage::CopyPassword,
            ],
            colors,
        ));
    }

    if fields.is_empty() {
        fields.push(
            text("No credentials")
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }

    card_with_margin(styled_card(column(fields).spacing(16).into()))
}

fn first_login_uri(login: &LoginView) -> Option<&str> {
    login
        .uris
        .as_ref()
        .and_then(|uris| uris.first())
        .and_then(|u| u.uri.as_deref())
}

// ---------------------------------------------------------------------------
// Card card
// ---------------------------------------------------------------------------

fn card_details_card<'a>(
    card: &'a CardView,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> = Vec::new();
    push_optional_field(
        &mut fields,
        "Cardholder name",
        card.cardholder_name.as_deref(),
        colors,
    );
    push_optional_field(&mut fields, "Brand", card.brand.as_deref(), colors);
    push_optional_field(&mut fields, "Number", card.number.as_deref(), colors);

    let expiration = match (card.exp_month.as_deref(), card.exp_year.as_deref()) {
        (Some(m), Some(y)) => Some(format!("{m}/{y}")),
        (Some(m), None) => Some(m.to_string()),
        (None, Some(y)) => Some(y.to_string()),
        (None, None) => None,
    };
    if let Some(exp) = expiration {
        fields.push(field_readonly("Expiration", exp, colors));
    }

    push_optional_field(&mut fields, "Security code", card.code.as_deref(), colors);

    if fields.is_empty() {
        fields.push(
            text("No card details")
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }
    card_with_margin(styled_card(column(fields).spacing(12).into()))
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
        fields.push(field_readonly("Name", full_name, colors));
    }

    push_optional_field(&mut fields, "Email", identity.email.as_deref(), colors);
    push_optional_field(&mut fields, "Phone", identity.phone.as_deref(), colors);
    push_optional_field(&mut fields, "Company", identity.company.as_deref(), colors);

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
        fields.push(field_readonly("Address", address_lines, colors));
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
        fields.push(field_readonly("City / region", locality, colors));
    }

    if fields.is_empty() {
        fields.push(
            text("No identity details")
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }
    card_with_margin(styled_card(column(fields).spacing(12).into()))
}

// ---------------------------------------------------------------------------
// SSH key card
// ---------------------------------------------------------------------------

fn ssh_key_card<'a>(
    key: &'a SshKeyView,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let fields = vec![
        field_readonly("Public key", &key.public_key, colors),
        field_readonly("Private key", &key.private_key, colors),
        field_readonly("Fingerprint", &key.fingerprint, colors),
    ];
    card_with_margin(styled_card(column(fields).spacing(12).into()))
}

// ---------------------------------------------------------------------------
// Autofill card
// ---------------------------------------------------------------------------

fn autofill_card<'a>(uri: &'a str, colors: &AppColors) -> Element<'a, DetailPaneMessage, AppTheme> {
    let label = text("Website").size(12).color(colors.text_muted);
    let value = text(uri).size(14).color(colors.text_primary);

    let copy_btn = icon_button(icons::BWI_COPY, DetailPaneMessage::CopyUrl, colors);
    let open_btn = icon_button(icons::BWI_EXTERNAL_LINK, DetailPaneMessage::OpenUrl, colors);

    let field_row = row![
        column![label, value].spacing(2).width(Fill),
        copy_btn,
        open_btn,
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    card_with_margin(styled_card(field_row.into()))
}

// ---------------------------------------------------------------------------
// Bottom bar
// ---------------------------------------------------------------------------

fn bottom_bar<'a>(colors: &AppColors) -> Element<'a, DetailPaneMessage, AppTheme> {
    let edit_btn = buttons::primary(text("Edit").size(14))
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
    label: &'a str,
    value: Option<&'a str>,
    colors: &AppColors,
) {
    if let Some(v) = value {
        fields.push(field_readonly(label, v, colors));
    }
}

fn field_with_action<'a>(
    label: &'a str,
    value: &'a str,
    icons_list: &[icons::BwiIcon],
    msgs: &[DetailPaneMessage],
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let buttons: Vec<Element<'a, DetailPaneMessage, AppTheme>> = icons_list
        .iter()
        .zip(msgs.iter())
        .map(|(icon, msg)| icon_button(*icon, msg.clone(), colors))
        .collect();

    let buttons_row = row(buttons).spacing(2).align_y(Alignment::Center);

    row![
        column![
            text(label).size(12).color(colors.text_muted),
            text(value).size(14).color(colors.text_primary),
        ]
        .spacing(2)
        .width(Fill),
        buttons_row,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}
