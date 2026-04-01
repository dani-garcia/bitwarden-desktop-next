use iced::widget::{Column, button, column, container, row, rule, text};
use iced::{Alignment, Element, Fill, Font, Length, Padding};

use crate::icons;
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

    let nav_button =
        |label: &'a str, icon: icons::Icon, filter: SidebarFilter| -> Element<'a, SidebarMessage> {
            let is_selected = active_filter == filter;
            let bg = if is_selected {
                theme::SELECTED_BG
            } else {
                iced::Color::TRANSPARENT
            };
            let color = if is_selected {
                theme::TEXT_PRIMARY
            } else {
                theme::TEXT_SECONDARY
            };

            button(
                row![icon.render(14.0, color), text(label).size(13).color(color),]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .on_press(SidebarMessage::FilterSelected(filter))
            .padding(Padding::from([6.0, 12.0]))
            .width(Fill)
            .style(move |_theme, status| {
                let bg_color = match status {
                    button::Status::Hovered if !is_selected => theme::ITEM_HOVER,
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
        section(vec![nav_button(
            "All items",
            icons::COLLECTION,
            SidebarFilter::AllItems,
        )]),
        section_header("TYPES"),
        section(vec![
            nav_button("Favorites", icons::STAR, SidebarFilter::Favorites),
            nav_button(
                "Login",
                icons::GLOBE,
                SidebarFilter::Category(CipherCategory::Login),
            ),
            nav_button(
                "Card",
                icons::CREDIT_CARD,
                SidebarFilter::Category(CipherCategory::Card),
            ),
            nav_button(
                "Identity",
                icons::PERSON,
                SidebarFilter::Category(CipherCategory::Identity),
            ),
            nav_button(
                "Secure note",
                icons::STICKY,
                SidebarFilter::Category(CipherCategory::SecureNote),
            ),
            nav_button(
                "SSH key",
                icons::KEY,
                SidebarFilter::Category(CipherCategory::SshKey),
            ),
        ]),
        rule::horizontal(1),
        section(vec![nav_button(
            "Trash",
            icons::TRASH,
            SidebarFilter::Trash,
        )]),
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
