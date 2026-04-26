pub(crate) mod account_switcher;
pub(crate) mod bottom_sheet;
pub(crate) mod buttons;
pub(crate) mod collapsible_pane;
pub(crate) mod drop_down;
pub(crate) mod fade_in_out;
pub(crate) mod icons;
pub(crate) mod inputs;
pub(crate) mod modal;
pub(crate) mod sidebar;
pub(crate) mod spinner;
pub(crate) mod toast;
pub(crate) mod totp;
pub(crate) mod virtual_list;

pub(crate) use fade_in_out::FadeInOut;

use iced::{
    Color, Element, Fill, Padding, Shadow,
    widget::{container, rule},
};

use crate::theme::{AppTheme, RADIUS_LG};

pub fn separator_h<'a, M: 'a>() -> Element<'a, M, AppTheme> {
    rule::horizontal(1)
        .style(|theme: &AppTheme| rule::Style {
            color: theme.colors.border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        })
        .into()
}

pub fn separator_v<'a, M: 'a>() -> Element<'a, M, AppTheme> {
    rule::vertical(1)
        .style(|theme: &AppTheme| rule::Style {
            color: theme.colors.border,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        })
        .into()
}

pub fn styled_card<'a, M: 'a>(content: Element<'a, M, AppTheme>) -> Element<'a, M, AppTheme> {
    container(content)
        .padding([12, 16])
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(iced::border::rounded(RADIUS_LG))
                .shadow(Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                })
        })
        .into()
}

pub fn card_with_margin<'a, M: 'a>(card: Element<'a, M, AppTheme>) -> Element<'a, M, AppTheme> {
    container(card)
        .padding(Padding {
            top: 0.0,
            right: 0.0,
            bottom: 8.0,
            left: 0.0,
        })
        .into()
}
