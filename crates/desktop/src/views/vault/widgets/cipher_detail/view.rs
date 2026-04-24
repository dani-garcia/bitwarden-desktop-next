//! Top-level read-only detail pane: header + scrollable body dispatching on
//! cipher type + bottom bar. Parallel to `cipher_edit::view`.

use bitwarden_vault::{CipherType, CipherView};
use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, row, scrollable, text},
};

use crate::{
    components::{self, buttons, icons},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    CipherDetailMessage,
    sections::{card, identity, login, shared, ssh_key},
};
use crate::views::vault::widgets::field_helpers::{card_with_margin, section_label, styled_card};

pub fn view<'a>(
    item: &'a CipherView,
    colors: &'a AppColors,
    top_radius: f32,
) -> Element<'a, CipherDetailMessage, AppTheme> {
    let header = header_row(item, colors);

    let mut sections: Vec<Element<'a, CipherDetailMessage, AppTheme>> = vec![
        section_label(fl!("detail-section-item-details"), colors),
        shared::item_details_card(item, colors),
    ];

    match item.r#type {
        CipherType::Login => {
            if let Some(login_view) = item.login.as_ref() {
                sections.push(section_label(
                    fl!("detail-section-login-credentials"),
                    colors,
                ));
                sections.push(login::login_card(login_view, colors));

                let uris = login::collect_login_uris(login_view);
                if !uris.is_empty() {
                    sections.push(section_label(
                        fl!("detail-section-autofill-options"),
                        colors,
                    ));
                    sections.push(login::autofill_card(&uris, colors));
                }
            }
        }
        CipherType::Card => {
            if let Some(card_view) = item.card.as_ref() {
                sections.push(section_label(fl!("detail-section-card-details"), colors));
                sections.push(card::card_details_card(card_view, colors));
            }
        }
        CipherType::Identity => {
            if let Some(identity_view) = item.identity.as_ref() {
                sections.push(section_label(
                    fl!("detail-section-personal-details"),
                    colors,
                ));
                sections.push(identity::identity_card(identity_view, colors));
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
                sections.push(ssh_key::ssh_key_card(key, colors));
            }
        }
    }

    if let Some(fields) = item.fields.as_deref().filter(|f| !f.is_empty()) {
        sections.push(section_label(fl!("detail-section-custom-fields"), colors));
        sections.push(shared::custom_fields_card(fields, colors));
    }

    let body = scrollable(column(sections).spacing(4).padding([12, 20])).height(Fill);
    let bottom = bottom_bar(colors);

    container(column![header, body, bottom].spacing(0).height(Fill))
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

fn header_row<'a>(
    item: &'a CipherView,
    colors: &AppColors,
) -> Element<'a, CipherDetailMessage, AppTheme> {
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
    .on_press(CipherDetailMessage::Close)
    .padding([1, 1]);

    // No background fill on the header — it inherits the parent container's
    // `card_bg`. A separate background here would mask the parent's rounded
    // top corners (iced doesn't clip children to parent border radius).
    let header =
        container(row![title, Space::new().width(Fill), close_btn].align_y(Alignment::Center))
            .padding([8, 20]);

    column![header, components::separator_h()].spacing(0).into()
}

fn bottom_bar<'a>(colors: &AppColors) -> Element<'a, CipherDetailMessage, AppTheme> {
    let edit_btn = buttons::primary(text(fl!("detail-edit-button")).size(14))
        .on_press(CipherDetailMessage::Edit)
        .padding([8, 20]);

    let delete_btn = buttons::ghost_icon(
        icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
        colors.item_hover,
    )
    .on_press(CipherDetailMessage::Delete)
    .padding([6, 6]);

    let bar =
        container(row![edit_btn, Space::new().width(Fill), delete_btn].align_y(Alignment::Center))
            .padding([8, 20])
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.background)
            });

    column![components::separator_h(), bar].spacing(0).into()
}
