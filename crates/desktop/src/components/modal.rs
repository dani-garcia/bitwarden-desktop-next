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
//!
//! Most callers should reach for [`dialog`] — it bundles the standard
//! card shape (rounded corners + drop shadow + dismiss-on-backdrop-click)
//! and only asks the caller for a body, a size, and a background token.
//! [`view`] is the lower-level primitive for non-card overlays.

use iced::{
    Alignment, Border, Color, Element, Fill, Length, Padding, Shadow, Vector,
    widget::{Space, center, column, container, mouse_area, opaque, row, stack, text},
};

use crate::{
    components::buttons,
    theme::{AppColors, AppTheme, RADIUS_LG},
};

/// Scrim color painted over the underlying view.
const BACKDROP: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.45);

/// Drop shadow shared by every modal dialog so the dialog reads as lifted
/// off the backdrop. Beefier than the per-card shadow inside the cipher
/// detail / generator option cards (this sits on top of the 45 % black
/// scrim, so it needs more weight to be felt).
const DIALOG_SHADOW: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
    offset: Vector::new(0.0, 4.0),
    blur_radius: 48.0,
};

/// Wrap `dialog` as a centered modal with a full-window darkened backdrop.
/// Clicks on the exposed backdrop fire `on_dismiss`; clicks on the dialog
/// itself are absorbed by z-order. The caller's `dialog` is responsible
/// for its own background, padding, and border radius.
///
/// The whole overlay is wrapped in [`opaque`] so mouse moves and hovers
/// can't reach widgets beneath — otherwise buttons below would still light
/// up under the scrim, suggesting they're interactive.
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

    opaque(stack![backdrop, centered].width(Fill).height(Fill))
}

/// Centered card-shaped modal — the default shape for app dialogs.
///
/// Bundles the shared visual treatment (rounded corners, drop shadow) and
/// the standard backdrop+dismiss behavior. Each call site picks its own
/// `bg` token via the closure so dialogs that look like content cards
/// (`card_bg`) and dialogs that look like surfaces (`background`) can
/// coexist without forcing a single palette.
///
/// `height: None` lets the dialog shrink to its content height — handy
/// for confirmation dialogs whose size depends on the body text.
pub fn dialog<'a, M>(
    width: f32,
    height: Option<f32>,
    bg: impl Fn(&AppColors) -> Color + Copy + 'static,
    body: Element<'a, M, AppTheme>,
    on_dismiss: M,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let card = container(body)
        .width(Length::Fixed(width))
        .height(height.map(Length::Fixed).unwrap_or(Length::Shrink))
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(bg(&theme.colors))
                .border(Border::default().rounded(RADIUS_LG))
                .shadow(DIALOG_SHADOW)
        });
    view(card.into(), on_dismiss)
}

/// Confirmation dialog with a title, body text, and primary/secondary
/// button row (e.g. delete-this-item flows). Caller resolves its own
/// fluent strings so the wording stays domain-specific.
///
/// Backdrop click + the cancel button both fire `on_cancel`. Width is
/// fixed at 380 px and the dialog shrinks to its content height.
pub fn confirm_dialog<'a, M>(
    title_text: impl Into<String>,
    body_text: impl Into<String>,
    confirm_label: impl Into<String>,
    cancel_label: impl Into<String>,
    on_confirm: M,
    on_cancel: M,
    colors: &AppColors,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let title = text(title_text.into())
        .size(18)
        .color(colors.text_primary)
        .font(crate::APP_FONT_BOLD);
    let body = text(body_text.into())
        .size(14)
        .color(colors.text_primary);

    let cancel_btn = buttons::secondary(text(cancel_label.into()).size(14))
        .on_press(on_cancel.clone())
        .padding([8, 20]);
    let confirm_btn = buttons::primary(text(confirm_label.into()).size(14))
        .on_press(on_confirm)
        .padding([8, 20]);

    let inner = column![
        title,
        body,
        row![Space::new().width(Fill), cancel_btn, confirm_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    ]
    .spacing(12)
    .padding(Padding::from([16, 20]))
    .width(Fill);

    dialog(380.0, None, |c| c.card_bg, inner.into(), on_cancel)
}
