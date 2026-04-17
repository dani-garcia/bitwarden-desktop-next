use iced::{
    Background, Border, Color, Element, Fill,
    widget::{container, text_input},
};

use crate::{components::icons, theme::AppTheme};

#[derive(Debug, Clone)]
pub enum SearchMessage {
    QueryChanged(String),
}

pub fn view<'a>(query: &'a str) -> Element<'a, SearchMessage, AppTheme> {
    let input = text_input("Search", query)
        .id("vault-search")
        .on_input(SearchMessage::QueryChanged)
        .icon(icons::SEARCH.input_icon(14.0, text_input::Side::Left))
        .size(14)
        .padding([4, 4])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_muted,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });

    container(input)
        .style(|theme: &AppTheme| {
            container::Style::default().border(
                Border::default()
                    .color(theme.colors.border)
                    .width(1.0)
                    .rounded(4),
            )
        })
        .padding([4, 16])
        .width(Fill)
        .into()
}
