//! `view()` entry point + top-level header and bottom-bar composition.

use bitwarden_vault::CipherType;
use iced::{
    Alignment, Element, Fill,
    widget::{Space, column, container, row, scrollable, text},
};

use crate::{
    components::{self, buttons, icons, modal},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{message::CipherEditMessage, sections, state::CipherForm};

pub fn view<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
    top_radius: f32,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let header = header_row(form, colors);

    let mut sections: Vec<Element<'a, CipherEditMessage, AppTheme>> = vec![
        sections::shared::section_label(fl!("form-section-item-details"), colors),
        sections::shared::item_details_card(form, colors),
    ];

    match form.modified.r#type {
        CipherType::Login => {
            sections.push(sections::shared::section_label(
                fl!("form-section-login-credentials"),
                colors,
            ));
            sections.push(sections::login::login_card(form, colors));
            sections.push(sections::shared::section_label(
                fl!("form-section-autofill-options"),
                colors,
            ));
            sections.push(sections::login::autofill_card(form, colors));
        }
        CipherType::Card => {
            sections.push(sections::shared::section_label(
                fl!("form-section-card-details"),
                colors,
            ));
            sections.push(sections::card::card_details_card(form, colors));
        }
        CipherType::Identity => {
            sections.push(sections::shared::section_label(
                fl!("form-section-personal-details"),
                colors,
            ));
            sections.push(sections::identity::identity_personal_card(form, colors));
            sections.push(sections::shared::section_label(
                fl!("form-section-identification"),
                colors,
            ));
            sections.push(sections::identity::identity_identification_card(
                form, colors,
            ));
            sections.push(sections::shared::section_label(
                fl!("form-section-contact-info"),
                colors,
            ));
            sections.push(sections::identity::identity_contact_card(form, colors));
            sections.push(sections::shared::section_label(
                fl!("form-section-address"),
                colors,
            ));
            sections.push(sections::identity::identity_address_card(form, colors));
        }
        CipherType::SecureNote => { /* notes live in the shared "Additional options" card below */ }
        CipherType::SshKey => {
            sections.push(sections::shared::section_label(
                fl!("form-section-ssh-key"),
                colors,
            ));
            sections.push(sections::ssh_key::ssh_key_card(form, colors));
        }
        // TODO(bank-account): no type-specific section yet — only the shared
        // additional-options + custom-fields cards render. See docs/todo.md.
        CipherType::BankAccount => {}
    }

    sections.push(sections::shared::section_label(
        fl!("form-section-additional-options"),
        colors,
    ));
    sections.push(sections::shared::additional_options_card(form, colors));

    sections.push(sections::shared::section_label(
        fl!("form-section-custom-fields"),
        colors,
    ));
    sections.push(sections::shared::custom_fields_card(form, colors));

    let body = scrollable(column(sections).spacing(4).padding([12, 20])).height(Fill);
    let bottom_bar = bottom_bar(form, colors);

    components::rounded_top_pane(
        column![header, body, bottom_bar].spacing(0).height(Fill),
        top_radius,
    )
}

fn header_row<'a>(
    form: &'a CipherForm,
    colors: &AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let title_text = if form.original.is_some() {
        match form.modified.r#type {
            CipherType::Login => fl!("form-title-edit-login"),
            CipherType::Card => fl!("form-title-edit-card"),
            CipherType::Identity => fl!("form-title-edit-identity"),
            CipherType::SecureNote => fl!("form-title-edit-note"),
            CipherType::SshKey => fl!("form-title-edit-ssh-key"),
            CipherType::BankAccount => fl!("form-title-edit-bank-account"),
        }
    } else {
        fl!("form-title-new-item")
    };

    let title = text(title_text).size(18).color(colors.text_primary);

    let cancel_btn = buttons::ghost_icon(
        icons::BWI_CLOSE.render(32.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(CipherEditMessage::Cancel)
    .padding([1, 1]);

    let header =
        container(row![title, Space::new().width(Fill), cancel_btn].align_y(Alignment::Center))
            .padding([8, 20]);

    column![header, components::separator_h()].spacing(0).into()
}

fn bottom_bar<'a>(
    form: &'a CipherForm,
    _colors: &AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let save_label = if form.saving {
        fl!("form-saving")
    } else {
        fl!("form-save")
    };
    let on_save = (!form.saving).then_some(CipherEditMessage::Save);
    let actions = modal::footer_actions(
        save_label,
        on_save,
        fl!("form-cancel"),
        CipherEditMessage::Cancel,
    );

    let bar = container(actions)
        .width(Fill)
        .padding([8, 20])
        .style(|theme: &AppTheme| container::Style::default().background(theme.colors.background));

    column![components::separator_h(), bar].spacing(0).into()
}
