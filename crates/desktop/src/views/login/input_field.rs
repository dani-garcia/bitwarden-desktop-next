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
#[expect(clippy::too_many_arguments)] // View-composition primitive; all args are structural.
pub fn floating_label_input<'a>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> LoginMessage + 'a,
    on_submit: Option<LoginMessage>,
    secure: bool,
    show_toggle: Option<(bool, LoginMessage)>,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let mut input = text_input("", value)
        .size(16)
        .padding([10, 12])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_secondary,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });

    // Skipping `.on_input` leaves the text_input read-only (iced renders it
    // as non-editable). We also drop `.on_submit` so Enter-spam during the
    // unlock is ignored at the widget layer.
    if !disabled {
        input = input.on_input(on_input);
        if let Some(submit_msg) = on_submit {
            input = input.on_submit(submit_msg);
        }
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

            let mut toggle_button =
                buttons::ghost_icon(toggle_icon, colors.item_hover).padding([10, 12]);
            if !disabled {
                toggle_button = toggle_button.on_press(toggle_msg);
            }

            row![input, toggle_button].align_y(Alignment::Center).into()
        } else {
            input.into()
        };

    let floating_label = container(text(label).size(14).color(colors.text_secondary))
        .padding([0, 4])
        .style(|theme: &AppTheme| {
            container::Style::default().background(theme.colors.background)
        });

    let input_border = container(input_row).width(Fill).style(|theme: &AppTheme| {
        container::Style::default().border(
            Border::default()
                .color(theme.colors.border)
                .width(1.0)
                .rounded(4),
        )
    });

    stack![
        column![Space::new().height(Length::Fixed(8.0)), input_border],
        container(floating_label).padding([0, 12]),
    ]
    .into()
}
