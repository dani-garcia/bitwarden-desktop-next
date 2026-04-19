//! Centered modal overlay with a darkening backdrop.
//!
//! Iced-idiomatic pattern lifted from the upstream modal example:
//! `stack![backdrop, centered(dialog)]`. The backdrop is a `mouse_area`
//! so clicks outside the dialog fire `on_dismiss`. The dialog itself
//! owns its own background and border radius (same rule as `bottom_sheet`:
//! iced doesn't clip children to parent border radius).
//!
//! Compose at the App level — see CLAUDE.md's "Hoist window-level overlays"
//! note — so the overlay covers the sidebar and title bar.

use iced::{
    Color, Element, Fill, Length,
    widget::{Space, center, container, mouse_area, stack},
};

use crate::theme::AppTheme;

/// Scrim color painted over the underlying view.
const BACKDROP: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.45);

/// Wrap `dialog` as a centered modal with a full-window darkened backdrop.
/// Clicks on the exposed backdrop fire `on_dismiss`; clicks on the dialog
/// itself are absorbed by z-order. The caller's `dialog` is responsible
/// for its own background, padding, and border radius.
pub fn view<'a, Message: Clone + 'a>(
    dialog: Element<'a, Message, AppTheme>,
    on_dismiss: Message,
) -> Element<'a, Message, AppTheme> {
    let backdrop = mouse_area(
        container(Space::new())
            .width(Fill)
            .height(Fill)
            .style(|_theme: &AppTheme| container::Style::default().background(BACKDROP)),
    )
    .on_press(on_dismiss);

    let centered = center(dialog).width(Length::Fill).height(Length::Fill);

    stack![backdrop, centered].width(Fill).height(Fill).into()
}
