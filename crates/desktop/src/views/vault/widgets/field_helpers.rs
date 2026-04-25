//! Small visual primitives shared between `cipher_detail` (read-only view)
//! and `cipher_edit` (editable view). Extracted so both can wear the same
//! section/card/label styling without duplicating the style closures.

use iced::{
    Element,
    widget::text,
};

use crate::{
    components,
    theme::{AppColors, AppTheme},
};

/// Section label (14 px, primary text color). Sits above a card. Accepts
/// `impl Into<String>` so either a static literal or an owned `fl!()` value
/// can be passed — the resulting widget owns the label.
pub fn section_label<'a, M: 'a>(
    label: impl Into<String>,
    colors: &AppColors,
) -> Element<'a, M, AppTheme> {
    text(label.into())
        .size(14)
        .color(colors.text_primary)
        .into()
}

/// Read-only `{label, value}` stacked pair. Used for displayed-only fields
/// that can wrap naturally — names, notes, multi-line addresses.
pub fn field_readonly<'a, M: 'a>(
    label: impl Into<String>,
    value: impl iced::widget::text::IntoFragment<'a>,
    colors: &AppColors,
) -> Element<'a, M, AppTheme> {
    iced::widget::column![
        text(label.into()).size(12).color(colors.text_muted),
        text(value).size(14).color(colors.text_primary),
    ]
    .spacing(2)
    .into()
}

/// Wraps a card element with bottom margin for section spacing. Thin alias
/// for `components::card_with_margin` — kept here so callers that already
/// import `field_helpers` don't need a second import path.
pub fn card_with_margin<'a, M: 'a>(card: Element<'a, M, AppTheme>) -> Element<'a, M, AppTheme> {
    components::card_with_margin(card)
}

/// Styled card wrapper. Thin alias for `components::styled_card` — exposed
/// here so callers that already import `field_helpers` don't need a second
/// import path. Named `styled_card` (not `card`) to avoid shadowing in
/// scopes that use `card` as a parameter name (e.g. `cipher_detail`'s
/// `card_details_card(card: &CardView)`).
pub fn styled_card<'a, M: 'a>(content: Element<'a, M, AppTheme>) -> Element<'a, M, AppTheme> {
    components::styled_card(content)
}

/// Format a passkey's creation timestamp in local time using a short,
/// US-style date + 12-hour clock ("1/29/26, 9:26 PM") — matches the
/// convention the official Bitwarden clients use. Shared by both the
/// detail pane (readonly row) and the cipher form (editable row).
pub fn format_passkey_date(dt: chrono::DateTime<chrono::Utc>) -> String {
    dt.with_timezone(&chrono::Local)
        .format("%-m/%-d/%y, %-I:%M %p")
        .to_string()
}
