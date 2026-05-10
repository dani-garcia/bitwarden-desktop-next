//! Centered modal overlay with a darkening backdrop.
//!
//! Pattern: `stack![backdrop, centered(dialog)]`. The dialog owns its own
//! background and border radius (iced doesn't clip children to parent
//! border radius). Compose at the App level so the overlay covers the
//! sidebar and title bar.
//!
//! Most callers should reach for [`dialog`] — it bundles the standard card
//! shape (rounded corners + drop shadow + dismiss-on-backdrop-click).
//! [`view`] is the lower-level primitive for non-card overlays.

use iced::{
    Alignment, Border, Color, Element, Fill, Length, Padding, Shadow, Vector,
    widget::{Space, center, column, container, mouse_area, opaque, row, stack, text},
};

use crate::{
    components::{BACKDROP_ALPHA, buttons, icons},
    theme::{AppColors, AppTheme, RADIUS_LG},
};

/// Drop-shadow alpha at full open, modulated by `progress`.
const SHADOW_ALPHA: f32 = 0.45;

/// Vertical slide distance (px) the dialog travels during the open
/// transition. Faked via a top-spacer column — iced has no transform widget.
const SLIDE_OFFSET_PX: f32 = 24.0;

/// Wrap `dialog` as a centered modal with a full-window darkened backdrop.
/// Clicks on the exposed backdrop fire `on_dismiss`. The caller's `dialog`
/// owns its own background, padding, and border radius.
///
/// `progress` is the open animation phase, `0.0` (fully closed — caller
/// shouldn't render at all) → `1.0` (fully open). Pass `1.0` for
/// non-animated callers.
///
/// The whole overlay is wrapped in [`opaque`] so hovers can't reach widgets
/// beneath the scrim and falsely light them up.
pub fn view<'a, Message: Clone + 'a>(
    dialog: impl Into<Element<'a, Message, AppTheme>>,
    on_dismiss: Message,
    progress: f32,
) -> Element<'a, Message, AppTheme> {
    let progress = progress.clamp(0.0, 1.0);
    let backdrop_color = Color {
        a: progress * BACKDROP_ALPHA,
        ..Color::BLACK
    };
    let backdrop =
        mouse_area(container(Space::new()).width(Fill).height(Fill).style(
            move |_theme: &AppTheme| container::Style::default().background(backdrop_color),
        ))
        .on_press(on_dismiss);

    // `opaque` on the dialog absorbs clicks on its empty/non-interactive
    // areas so they don't fall through to the backdrop's dismiss handler.
    let slide_offset = (1.0 - progress) * SLIDE_OFFSET_PX;
    let slid = column![
        Space::new().height(Length::Fixed(slide_offset)),
        opaque(dialog.into()),
    ];

    let centered = center(slid).width(Length::Fill).height(Length::Fill);

    opaque(stack![backdrop, centered].width(Fill).height(Fill))
}

/// Centered card-shaped modal — the default shape for app dialogs.
/// Each call site picks its own `bg` token via the closure so dialogs that
/// look like content cards (`card_bg`) and dialogs that look like surfaces
/// (`background`) can coexist. `height: None` shrinks to content height.
pub fn dialog<'a, M>(
    width: f32,
    height: Option<f32>,
    bg: impl Fn(&AppColors) -> Color + Copy + 'static,
    progress: f32,
    body: impl Into<Element<'a, M, AppTheme>>,
    on_dismiss: M,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let progress = progress.clamp(0.0, 1.0);
    let shadow = Shadow {
        color: Color {
            a: progress * SHADOW_ALPHA,
            ..Color::BLACK
        },
        offset: Vector::new(0.0, 4.0),
        blur_radius: 48.0,
    };
    let card = container(body)
        .width(Length::Fixed(width))
        .height(height.map(Length::Fixed).unwrap_or(Length::Shrink))
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(bg(&theme.colors))
                .border(Border::default().rounded(RADIUS_LG))
                .shadow(shadow)
        });
    view(card, on_dismiss, progress)
}

/// Title row with a trailing close-X button — the standard header for
/// app dialog modals (export / import / new folder / generator / settings
/// content pane / generic 20-pt-titled modals). The X fires `on_close`.
pub fn dialog_header<'a, M>(
    title_text: impl Into<String>,
    on_close: M,
    colors: &AppColors,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    row![
        text(title_text.into())
            .size(20)
            .font(crate::APP_FONT_BOLD)
            .color(colors.text_primary),
        Space::new().width(Fill),
        buttons::ghost_icon(
            icons::X_LG.render(16.0, colors.text_primary),
            colors.item_hover,
        )
        .padding([6, 6])
        .on_press(on_close),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// Standard primary + secondary button row for app dialog footers
/// (export / import / new folder / login self-hosted modal). The primary
/// button is rendered enabled when `on_primary` is `Some(_)`, otherwise
/// it's disabled (no `on_press`). Buttons are left-aligned with 8 px gap.
pub fn footer_actions<'a, M>(
    primary_label: impl Into<String>,
    on_primary: Option<M>,
    secondary_label: impl Into<String>,
    on_secondary: M,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let mut primary_btn = buttons::primary(text(primary_label.into()).size(14)).padding([8, 20]);
    if let Some(msg) = on_primary {
        primary_btn = primary_btn.on_press(msg);
    }
    let secondary_btn = buttons::secondary(text(secondary_label.into()).size(14))
        .on_press(on_secondary)
        .padding([8, 20]);

    row![primary_btn, secondary_btn]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}

/// Confirmation dialog with a title, body text, and primary/secondary
/// button row. Backdrop click + cancel button both fire `on_cancel`. Width
/// is fixed at 380 px; height shrinks to content.
#[expect(
    clippy::too_many_arguments,
    reason = "explicit args read clearly at the call site; a struct here would be pure boilerplate"
)]
pub fn confirm_dialog<'a, M>(
    title_text: impl Into<String>,
    body_text: impl Into<String>,
    confirm_label: impl Into<String>,
    cancel_label: impl Into<String>,
    on_confirm: M,
    on_cancel: M,
    colors: &AppColors,
    progress: f32,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let title = text(title_text.into())
        .size(18)
        .color(colors.text_primary)
        .font(crate::APP_FONT_BOLD);
    let body = text(body_text.into()).size(14).color(colors.text_primary);

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

    dialog(380.0, None, |c| c.card_bg, progress, inner, on_cancel)
}
