use iced::{
    Alignment, Background, Element, Fill, Padding,
    widget::{Space, column, container, row, scrollable, text},
};

use crate::{
    components::{self, buttons, icons},
    state::CipherItem,
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
    item: &'a CipherItem,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let header = header_row(item, colors);

    let mut sections: Vec<Element<'a, DetailPaneMessage, AppTheme>> = vec![
        section_label("Item details", colors),
        item_details_card(item, colors),
        section_label("Login credentials", colors),
        credentials_card(item, colors),
    ];

    if item.url.is_some() {
        sections.push(section_label("Autofill options", colors));
        sections.push(autofill_card(item, colors));
    }

    let body = scrollable(column(sections).spacing(4).padding([12, 20])).height(Fill);
    let bottom_bar = bottom_bar(colors);

    let pane = container(column![header, body, bottom_bar].spacing(0).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(theme.colors.card_bg)),
            ..Default::default()
        });

    row![components::separator_v(), pane].height(Fill).into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn header_row<'a>(
    item: &'a CipherItem,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let category_label = match item.category {
        crate::state::CipherCategory::Login => "View login",
        crate::state::CipherCategory::Card => "View card",
        crate::state::CipherCategory::Identity => "View identity",
        crate::state::CipherCategory::SecureNote => "View note",
        crate::state::CipherCategory::SshKey => "View SSH key",
    };

    let title = text(category_label).size(18).color(colors.text_primary);

    let close_btn = buttons::ghost_icon(
        icons::BWI_CLOSE.render(32.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(DetailPaneMessage::Close)
    .padding([1, 1]);

    let header =
        container(row![title, Space::new().width(Fill), close_btn].align_y(Alignment::Center))
            .padding([8, 20])
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.background)),
                ..Default::default()
            });

    column![header, components::separator_h()].spacing(0).into()
}

// ---------------------------------------------------------------------------
// Section label
// ---------------------------------------------------------------------------

fn section_label<'a>(
    label: &'a str,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    text(label).size(14).color(colors.text_primary).into()
}

// ---------------------------------------------------------------------------
// Item details card
// ---------------------------------------------------------------------------

fn item_details_card<'a>(
    item: &'a CipherItem,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let name_field = field_readonly("Name", &item.name, colors);
    card_with_margin(components::styled_card(
        column![name_field].spacing(12).into(),
    ))
}

// ---------------------------------------------------------------------------
// Credentials card
// ---------------------------------------------------------------------------

fn credentials_card<'a>(
    item: &'a CipherItem,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let mut fields: Vec<Element<'a, DetailPaneMessage, AppTheme>> = Vec::new();

    if let Some(ref username) = item.username {
        fields.push(field_with_action(
            "Username",
            username,
            &[icons::BWI_COPY],
            &[DetailPaneMessage::CopyUsername],
            colors,
        ));
    }

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

    if fields.is_empty() {
        fields.push(
            text("No credentials")
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    }

    card_with_margin(components::styled_card(column(fields).spacing(16).into()))
}

// ---------------------------------------------------------------------------
// Autofill card
// ---------------------------------------------------------------------------

fn autofill_card<'a>(
    item: &'a CipherItem,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    let url = item.url.as_deref().unwrap_or("");

    let label = text("Website").size(12).color(colors.text_muted);
    let value = text(url).size(14).color(colors.text_primary);

    let copy_btn = icon_button(icons::BWI_COPY, DetailPaneMessage::CopyUrl, colors);
    let open_btn = icon_button(icons::BWI_EXTERNAL_LINK, DetailPaneMessage::OpenUrl, colors);

    let field_row = row![
        column![label, value].spacing(2).width(Fill),
        copy_btn,
        open_btn,
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    card_with_margin(components::styled_card(field_row.into()))
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
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.background)),
                ..Default::default()
            });

    column![components::separator_h(), bar].spacing(0).into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn field_readonly<'a>(
    label: &'a str,
    value: &'a str,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    column![
        text(label).size(12).color(colors.text_muted),
        text(value).size(14).color(colors.text_primary),
    ]
    .spacing(2)
    .into()
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

fn icon_button<'a>(
    icon: icons::BwiIcon,
    msg: DetailPaneMessage,
    colors: &AppColors,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    buttons::ghost_icon(icon.render(18.0, colors.text_primary), colors.item_hover)
        .on_press(msg)
        .padding([6, 6])
        .into()
}

/// Wraps a card element with bottom margin for section spacing.
fn card_with_margin<'a>(
    card: Element<'a, DetailPaneMessage, AppTheme>,
) -> Element<'a, DetailPaneMessage, AppTheme> {
    container(card)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 8.0,
            left: 0.0,
        })
        .into()
}
