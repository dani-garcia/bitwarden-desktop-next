use iced::widget::{Space, button, column, container, row, rule, scrollable, text};
use iced::{Alignment, Element, Fill, Font, Padding};

use crate::icons;
use crate::state::CipherItem;
use crate::theme;

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

pub fn view<'a>(item: &'a CipherItem) -> Element<'a, DetailPaneMessage> {
    let header = header_row(item);

    let mut sections: Vec<Element<'a, DetailPaneMessage>> = vec![
        section_label("Item details"),
        item_details_card(item),
        section_label("Login credentials"),
        credentials_card(item),
    ];

    // --- Autofill options section ---
    if item.url.is_some() {
        sections.push(section_label("Autofill options"));
        sections.push(autofill_card(item));
    }

    let body = scrollable(column(sections).spacing(4).padding([12, 20])).height(Fill);

    let bottom_bar = bottom_bar();

    let vert_separator = rule::vertical(1).style(|_theme| rule::Style {
        color: theme::BORDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    });

    let pane = container(column![header, body, bottom_bar].spacing(0).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::CARD_BG)),
            ..Default::default()
        });

    row![vert_separator, pane].height(Fill).into()
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn header_row<'a>(item: &'a CipherItem) -> Element<'a, DetailPaneMessage> {
    let category_label = match item.category {
        crate::state::CipherCategory::Login => "View login",
        crate::state::CipherCategory::Card => "View card",
        crate::state::CipherCategory::Identity => "View identity",
        crate::state::CipherCategory::SecureNote => "View note",
        crate::state::CipherCategory::SshKey => "View SSH key",
    };

    let title = text(category_label)
        .size(18)
        .color(theme::TEXT_PRIMARY)
        .font(Font {
            weight: iced::font::Weight::Bold,
            ..Font::DEFAULT
        });

    let close_btn = button(icons::BWI_CLOSE.render(32.0, theme::TEXT_SECONDARY))
        .on_press(DetailPaneMessage::Close)
        .padding([1, 1])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => theme::ITEM_HOVER,
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TEXT_SECONDARY,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

    let header =
        container(row![title, Space::new().width(Fill), close_btn].align_y(Alignment::Center))
            .padding([8, 20])
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(theme::BACKGROUND)),
                ..Default::default()
            });

    let divider = rule::horizontal(1).style(|_theme| rule::Style {
        color: theme::BORDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    });

    column![header, divider].spacing(0).into()
}

// ---------------------------------------------------------------------------
// Section label
// ---------------------------------------------------------------------------

fn section_label<'a>(label: &'a str) -> Element<'a, DetailPaneMessage> {
    text(label).size(14).color(theme::TEXT_PRIMARY).into()
}

// ---------------------------------------------------------------------------
// Item details card
// ---------------------------------------------------------------------------

fn item_details_card<'a>(item: &'a CipherItem) -> Element<'a, DetailPaneMessage> {
    let name_field = field_readonly("Name", &item.name);

    styled_card(column![name_field].spacing(12).into())
}

// ---------------------------------------------------------------------------
// Credentials card
// ---------------------------------------------------------------------------

fn credentials_card<'a>(item: &'a CipherItem) -> Element<'a, DetailPaneMessage> {
    let mut fields: Vec<Element<'a, DetailPaneMessage>> = Vec::new();

    if let Some(ref username) = item.username {
        fields.push(field_with_action(
            "Username",
            username,
            &[icons::BWI_COPY],
            &[DetailPaneMessage::CopyUsername],
        ));
    }

    fields.push(field_with_action(
        "Password",
        "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}",
        &[icons::BWI_EYE, icons::BWI_COPY],
        &[DetailPaneMessage::TogglePasswordVisibility, DetailPaneMessage::CopyPassword],
    ));

    if fields.is_empty() {
        fields.push(
            text("No credentials")
                .size(13)
                .color(theme::TEXT_MUTED)
                .into(),
        );
    }

    styled_card(column(fields).spacing(16).into())
}

// ---------------------------------------------------------------------------
// Autofill card
// ---------------------------------------------------------------------------

fn autofill_card<'a>(item: &'a CipherItem) -> Element<'a, DetailPaneMessage> {
    let url = item.url.as_deref().unwrap_or("");

    let label = text("Website").size(11).color(theme::TEXT_MUTED);
    let value = text(url).size(14).color(theme::TEXT_PRIMARY);

    let copy_btn = icon_button(icons::BWI_COPY, DetailPaneMessage::CopyUrl);
    let open_btn = icon_button(icons::BWI_EXTERNAL_LINK, DetailPaneMessage::OpenUrl);

    let field_row = row![
        column![label, value].spacing(2).width(Fill),
        copy_btn,
        open_btn,
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    styled_card(field_row.into())
}

// ---------------------------------------------------------------------------
// Bottom bar
// ---------------------------------------------------------------------------

fn bottom_bar<'a>() -> Element<'a, DetailPaneMessage> {
    let divider = rule::horizontal(1).style(|_theme| rule::Style {
        color: theme::BORDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    });

    let edit_btn = button(text("Edit").size(14).color(theme::CARD_BG))
        .on_press(DetailPaneMessage::Edit)
        .padding([8, 20])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => theme::ACCENT,
                _ => theme::BUTTON_PRIMARY,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: iced::Border {
                    radius: 20.0.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

    let delete_btn = button(icons::BWI_TRASH.render(18.0, theme::TITLEBAR_CLOSE_HOVER))
        .on_press(DetailPaneMessage::Delete)
        .padding([6, 6])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => theme::ITEM_HOVER,
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TITLEBAR_CLOSE_HOVER,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

    let bar =
        container(row![edit_btn, Space::new().width(Fill), delete_btn].align_y(Alignment::Center))
            .padding([8, 20])
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(theme::BACKGROUND)),
                ..Default::default()
            });

    column![divider, bar].spacing(0).into()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn field_readonly<'a>(label: &'a str, value: &'a str) -> Element<'a, DetailPaneMessage> {
    column![
        text(label).size(11).color(theme::TEXT_MUTED),
        text(value).size(14).color(theme::TEXT_PRIMARY),
    ]
    .spacing(2)
    .into()
}

fn field_with_action<'a>(
    label: &'a str,
    value: &'a str,
    icons_list: &[icons::BwiIcon],
    msgs: &[DetailPaneMessage],
) -> Element<'a, DetailPaneMessage> {
    let mut buttons: Vec<Element<'a, DetailPaneMessage>> = Vec::new();
    for (icon, msg) in icons_list.iter().zip(msgs.iter()) {
        buttons.push(icon_button(*icon, msg.clone()));
    }

    let buttons_row = row(buttons).spacing(2).align_y(Alignment::Center);

    row![
        column![
            text(label).size(11).color(theme::TEXT_MUTED),
            text(value).size(14).color(theme::TEXT_PRIMARY),
        ]
        .spacing(2)
        .width(Fill),
        buttons_row,
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

fn icon_button<'a>(icon: icons::BwiIcon, msg: DetailPaneMessage) -> Element<'a, DetailPaneMessage> {
    button(icon.render(18.0, theme::TEXT_PRIMARY))
        .on_press(msg)
        .padding([6, 6])
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => theme::ITEM_HOVER,
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                shadow: iced::Shadow::default(),
                snap: false,
            }
        })
        .into()
}

fn styled_card<'a>(content: Element<'a, DetailPaneMessage>) -> Element<'a, DetailPaneMessage> {
    container(
        container(content)
            .padding([12, 16])
            .width(Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Background::Color(theme::BACKGROUND)),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .padding(Padding {
        top: 0.0,
        right: 0.0,
        bottom: 8.0,
        left: 0.0,
    })
    .into()
}
