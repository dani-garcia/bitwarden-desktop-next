use iced::widget::{container, row, text_input};
use iced::{Alignment, Element, Fill, Padding};

use crate::icons;
use crate::theme;

#[derive(Debug, Clone)]
pub enum SearchMessage {
    QueryChanged(String),
}

pub fn view(query: &str) -> Element<'_, SearchMessage> {
    let search_icon = icons::SEARCH.render(14.0, theme::TEXT_MUTED);

    let input = text_input("Search vault", query)
        .on_input(SearchMessage::QueryChanged)
        .size(14)
        .padding(Padding::from([8.0, 8.0]))
        .width(Fill)
        .style(|_theme, _status| text_input::Style {
            background: iced::Background::Color(iced::Color::TRANSPARENT),
            border: iced::Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            icon: theme::TEXT_MUTED,
            placeholder: theme::TEXT_MUTED,
            value: theme::TEXT_PRIMARY,
            selection: theme::ACCENT,
        });

    container(
        row![search_icon, input]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding(Padding::from([0.0, 12.0])),
    )
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::INPUT_BG)),
        border: iced::Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .padding(Padding::from([8.0, 16.0]))
    .width(Fill)
    .into()
}
