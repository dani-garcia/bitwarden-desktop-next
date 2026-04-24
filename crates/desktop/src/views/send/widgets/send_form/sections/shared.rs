//! Primitives shared between the per-card section modules.

use iced::{
    Color, Element, Fill, Shadow,
    widget::{column, container, text},
};

use crate::{
    theme::{AppColors, AppTheme, RADIUS_LG},
    views::send::widgets::send_form::SendFormMessage,
};

/// Heading + rounded card body used by every section.
pub(in super::super) fn card_section<'a>(
    heading: String,
    items: Vec<Element<'a, SendFormMessage, AppTheme>>,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let body = container(column(items).spacing(12))
        .padding(16)
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(iced::Border::default().rounded(RADIUS_LG))
                .shadow(Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                })
        });

    column![
        text(heading)
            .size(14)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD),
        body,
    ]
    .spacing(8)
    .into()
}
