use iced::widget::{button, column, container, row, rule, text, Column};
use iced::{Element, Fill, Font, Length, Padding};

use crate::state::{CipherCategory, SidebarFilter};
use crate::theme;

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    FilterSelected(SidebarFilter),
}

pub fn view<'a>(active_filter: SidebarFilter) -> Element<'a, SidebarMessage> {
    let section = |items: Vec<Element<'a, SidebarMessage>>| -> Element<'a, SidebarMessage> {
        Column::with_children(items).spacing(2).into()
    };

    let nav_button = |label: &'a str, filter: SidebarFilter| -> Element<'a, SidebarMessage> {
        let is_selected = active_filter == filter;
        let bg = if is_selected {
            theme::SELECTED_BG
        } else {
            iced::Color::TRANSPARENT
        };

        button(
            text(label)
                .size(13)
                .color(if is_selected {
                    theme::TEXT_PRIMARY
                } else {
                    theme::TEXT_SECONDARY
                }),
        )
        .on_press(SidebarMessage::FilterSelected(filter))
        .padding(Padding::from([6.0, 12.0]))
        .width(Fill)
        .style(move |_theme, status| {
            let bg_color = match status {
                button::Status::Hovered => {
                    if is_selected {
                        bg
                    } else {
                        theme::ITEM_HOVER
                    }
                }
                _ => bg,
            };
            button::Style {
                background: Some(iced::Background::Color(bg_color)),
                text_color: theme::TEXT_PRIMARY,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            }
        })
        .into()
    };

    let section_header = |label: &'a str| -> Element<'a, SidebarMessage> {
        container(text(label).size(11).color(theme::TEXT_MUTED))
            .padding(Padding {
                top: 8.0,
                right: 12.0,
                bottom: 4.0,
                left: 12.0,
            })
            .into()
    };

    let logo_header = container(
        column![
            row![
                text("bit").size(18).color(theme::TEXT_PRIMARY).font(Font {
                    weight: iced::font::Weight::Bold,
                    ..Font::DEFAULT
                }),
                text("warden").size(18).color(theme::TEXT_PRIMARY),
            ],
            text("Password Manager")
                .size(10)
                .color(theme::TEXT_SECONDARY),
        ]
        .spacing(0),
    )
    .padding(Padding {
        top: 8.0,
        right: 12.0,
        bottom: 12.0,
        left: 12.0,
    });

    let content = column![
        logo_header,
        section_header("VAULT"),
        section(vec![nav_button("All items", SidebarFilter::AllItems)]),
        section_header("TYPES"),
        section(vec![
            nav_button("Favorites", SidebarFilter::Favorites),
            nav_button("Login", SidebarFilter::Category(CipherCategory::Login)),
            nav_button("Card", SidebarFilter::Category(CipherCategory::Card)),
            nav_button("Identity", SidebarFilter::Category(CipherCategory::Identity)),
            nav_button(
                "Secure note",
                SidebarFilter::Category(CipherCategory::SecureNote),
            ),
            nav_button("SSH key", SidebarFilter::Category(CipherCategory::SshKey)),
        ]),
        rule::horizontal(1),
        section(vec![nav_button("Trash", SidebarFilter::Trash)]),
    ]
    .spacing(2);

    container(content)
        .width(Length::Fixed(200.0))
        .height(Fill)
        .padding(Padding::from([8.0, 0.0]))
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into()
}
