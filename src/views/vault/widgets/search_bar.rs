use iced::{
    Alignment, Background, Border, Color, Element, Fill,
    widget::{container, row, text_input},
};

use crate::{components::icons, theme::{AppColors, AppTheme}};

#[derive(Debug, Clone)]
pub enum SearchMessage {
    QueryChanged(String),
}

pub fn view<'a>(query: &'a str, colors: &AppColors) -> Element<'a, SearchMessage, AppTheme> {
    let search_icon: Element<'_, SearchMessage, AppTheme> =
        icons::SEARCH.render(14.0, colors.text_muted);

    let input = text_input("Search", query)
        .id("vault-search")
        .on_input(SearchMessage::QueryChanged)
        .size(14)
        .padding([4, 4])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_muted,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });

    container(
        row![search_icon, input]
            .spacing(8)
            .align_y(Alignment::Center)
            .padding([0, 4]),
    )
    .style(|theme: &AppTheme| container::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border {
            color: theme.colors.border,
            width: 1.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    })
    .padding([4, 16])
    .width(Fill)
    .into()
}
