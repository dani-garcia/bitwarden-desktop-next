//! Reusable input primitives shared across views.
//!
//! For inputs that need a show/hide toggle (password, card CVV, SSN, hidden
//! custom fields), use [`crate::components::reveal_input`] — it keeps the
//! reveal state inside the widget tree instead of threading it through app
//! state. Both that module and [`floating_label_input`] compose through
//! [`floating_label_frame`] for a consistent visual idiom.

use iced::{
    Background, Border, Color, Element, Fill, Length,
    widget::{Space, column, container, stack, text, text_input, TextInput},
};

use crate::theme::{AppColors, AppTheme};

/// Preconfigured `text_input` matching our floating-label look:
/// transparent background + no border (the wrapping
/// [`floating_label_frame`] draws the border). Callers chain
/// `.on_input`/`.on_submit`/`.secure` as needed.
pub fn bare_text_input<'a, M: Clone + 'a>(value: &'a str) -> TextInput<'a, M, AppTheme> {
    text_input("", value)
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
        })
}

/// Wrap any content in the floating-label frame: a bordered container with
/// a small label chip stacked on top of the border. The chip has a
/// background matching the page, so the border visually breaks behind it.
///
/// Used by [`floating_label_input`] (plain text input),
/// [`crate::components::reveal_input`] (text input + eye toggle), and the
/// labeled pick_list wrappers in the cipher form.
pub fn floating_label_frame<'a, M: 'a>(
    label: &'a str,
    content: Element<'a, M, AppTheme>,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    let floating_label = container(text(label).size(14).color(colors.text_secondary))
        .padding([0, 4])
        .style(|theme: &AppTheme| container::Style::default().background(theme.colors.background));

    let bordered = container(content).width(Fill).style(|theme: &AppTheme| {
        container::Style::default().border(
            Border::default()
                .color(theme.colors.border)
                .width(1.0)
                .rounded(4),
        )
    });

    stack![
        column![Space::new().height(Length::Fixed(8.0)), bordered],
        container(floating_label).padding([0, 12]),
    ]
    .into()
}

/// Reusable floating-label text input.
///
/// Generic over the caller's message type `M` so both the login view and
/// the cipher edit form can reuse this widget.
pub fn floating_label_input<'a, M>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    on_submit: Option<M>,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let mut input = bare_text_input(value);

    // Skipping `.on_input` leaves the text_input read-only (iced renders it
    // as non-editable). We also drop `.on_submit` so Enter-spam during any
    // in-flight task is ignored at the widget layer.
    if !disabled {
        input = input.on_input(on_input);
        if let Some(submit_msg) = on_submit {
            input = input.on_submit(submit_msg);
        }
    }

    floating_label_frame(label, input.into(), colors)
}
