use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length,
    widget::{Space, column, container, row, stack, text, text_input},
};

use crate::{
    components::{buttons, icons},
    theme::{AppColors, AppTheme},
};

use super::LoginMessage;

/// Reusable floating-label text input with optional password toggle.
///
/// The pattern: a bordered container holds the text input (and optional eye toggle),
/// with a small label positioned on the top border via `stack`.
pub fn floating_label_input<'a>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> LoginMessage + 'a,
    on_submit: Option<LoginMessage>,
    secure: bool,
    show_toggle: Option<(bool, LoginMessage)>,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let mut input = text_input("", value)
        .on_input(on_input)
        .size(16)
        .padding([10, 12])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 0.0.into(),
            },
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_secondary,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });

    if let Some(submit_msg) = on_submit {
        input = input.on_submit(submit_msg);
    }
    if secure {
        input = input.secure(true);
    }

    let input_row: Element<'_, LoginMessage, AppTheme> =
        if let Some((is_visible, toggle_msg)) = show_toggle {
            let toggle_icon = if is_visible {
                icons::EYE
            } else {
                icons::EYE_SLASH
            }
            .render(16.0, colors.text_secondary);

            let toggle_button = buttons::ghost_icon(toggle_icon, colors.item_hover)
                .on_press(toggle_msg)
                .padding([10, 12]);

            row![input, toggle_button].align_y(Alignment::Center).into()
        } else {
            input.into()
        };

    let floating_label = container(text(label).size(14).color(colors.text_secondary))
        .padding([0, 4])
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(theme.colors.background)),
            ..Default::default()
        });

    let input_border =
        container(input_row)
            .width(Fill)
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    color: theme.colors.border,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

    stack![
        column![Space::new().height(Length::Fixed(8.0)), input_border],
        container(floating_label).padding([0, 12]),
    ]
    .into()
}
