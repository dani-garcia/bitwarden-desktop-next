use iced::widget::{container, text_input};
use iced::{Element, Fill, Padding};

use crate::theme;

#[derive(Debug, Clone)]
pub enum SearchMessage {
    QueryChanged(String),
}

pub fn view(query: &str) -> Element<'_, SearchMessage> {
    container(
        text_input("Search vault", query)
            .on_input(SearchMessage::QueryChanged)
            .size(14)
            .padding(Padding::from([8.0, 12.0]))
            .width(Fill)
            .style(|_theme, _status| text_input::Style {
                background: iced::Background::Color(theme::INPUT_BG),
                border: iced::Border {
                    color: theme::BORDER,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                icon: theme::TEXT_MUTED,
                placeholder: theme::TEXT_MUTED,
                value: theme::TEXT_PRIMARY,
                selection: theme::ACCENT,
            }),
    )
    .padding(Padding::from([8.0, 16.0]))
    .width(Fill)
    .into()
}
