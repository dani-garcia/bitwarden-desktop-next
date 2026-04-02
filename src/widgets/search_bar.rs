use iced::{
    Alignment, Background, Border, Color, Element, Fill,
    widget::{container, row, text_input},
};

use crate::{icons, theme};

#[derive(Debug, Clone)]
pub enum SearchMessage {
    QueryChanged(String),
}

pub fn view(query: &str) -> Element<'_, SearchMessage> {
    let search_icon = icons::SEARCH.render(14.0, theme::TEXT_MUTED);

    let input = text_input("Search", query)
        .id("vault-search")
        .on_input(SearchMessage::QueryChanged)
        .size(14)
        .padding([4, 4])
        .width(Fill)
        .style(|_theme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border {
                color: Color::TRANSPARENT,
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
            .padding([0, 4]),
    )
    .style(|_theme| container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border {
            color: theme::BORDER,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .padding([4, 16])
    .width(Fill)
    .into()
}
