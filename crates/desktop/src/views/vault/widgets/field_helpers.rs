//! Small visual primitives shared between `detail_pane` (read-only view)
//! and `cipher_form` (editable view). Extracted so both can wear the same
//! section/card/label styling without duplicating the style closures.

use iced::{
    Element, Padding,
    widget::{container, text},
};

use crate::{
    components::{self, buttons, icons},
    theme::{AppColors, AppTheme},
};

/// Section label (14 px, primary text color). Sits above a card.
pub fn section_label<'a, M: 'a>(label: &'a str, colors: &AppColors) -> Element<'a, M, AppTheme> {
    text(label).size(14).color(colors.text_primary).into()
}

/// Read-only `{label, value}` stacked pair. Used for displayed-only fields
/// (card fingerprints, SSH keys, notes, etc.) inside both panes.
pub fn field_readonly<'a, M: 'a>(
    label: &'a str,
    value: impl iced::widget::text::IntoFragment<'a>,
    colors: &AppColors,
) -> Element<'a, M, AppTheme> {
    iced::widget::column![
        text(label).size(12).color(colors.text_muted),
        text(value).size(14).color(colors.text_primary),
    ]
    .spacing(2)
    .into()
}

/// Small 18 px icon button with ghost-style hover.
pub fn icon_button<'a, M: 'a + Clone>(
    icon: icons::BwiIcon,
    msg: M,
    colors: &AppColors,
) -> Element<'a, M, AppTheme> {
    buttons::ghost_icon(icon.render(18.0, colors.text_primary), colors.item_hover)
        .on_press(msg)
        .padding([6, 6])
        .into()
}

/// Wraps a card element with bottom margin for section spacing.
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

/// Styled card wrapper. Thin alias for `components::styled_card` — exposed
/// here so callers that already import `field_helpers` don't need a second
/// import path. Named `styled_card` (not `card`) to avoid shadowing in
/// scopes that use `card` as a parameter name (e.g. `detail_pane`'s
/// `card_details_card(card: &CardView)`).
pub fn styled_card<'a, M: 'a>(content: Element<'a, M, AppTheme>) -> Element<'a, M, AppTheme> {
    components::styled_card(content)
}
