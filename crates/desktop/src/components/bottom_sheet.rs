use iced::{
    Color, Element, Fill, Length,
    widget::{Space, column, container, mouse_area, opaque, stack},
};

use crate::theme::AppTheme;

/// Below this window width (logical px) the right-hand pane (vault detail,
/// send form) collapses into a bottom sheet rendered over the list pane.
/// Single source of truth shared by every authenticated screen so they all
/// switch layout at the same width.
pub const SHEET_BREAKPOINT_PX: f32 = 1000.0;

/// Visible strip at the top of the underlying view above the sheet
/// (≈2× `TITLE_BAR_HEIGHT`). Lets the user keep some context of what's
/// behind the sheet.
pub const SHEET_TOP_INSET_PX: f32 = 64.0;

/// Top-corner radius of the sheet's content surface.
pub const SHEET_TOP_RADIUS_PX: f32 = 16.0;

/// Backdrop alpha at full open, modulated by `progress`.
const BACKDROP_ALPHA: f32 = 0.45;

/// How far below its rest position the sheet starts during the open
/// transition. Faked via animated top-padding inside the offset column —
/// iced has no transform widget.
const SLIDE_OFFSET_PX: f32 = 32.0;

/// Wrap `content` as a sheet that fills the available area starting
/// `top_inset` pixels below the top. Clicks on the exposed backdrop fire
/// `on_backdrop_click` if provided.
///
/// `progress` is the open-animation phase, `0.0` (fully closed — caller
/// shouldn't render at all) → `1.0` (fully open). Pass `1.0` for
/// non-animated callers.
///
/// The caller's content is responsible for its own background and rounded
/// corners — iced doesn't clip children to parent border radius.
pub fn view<'a, Message: Clone + 'a>(
    content: Element<'a, Message, AppTheme>,
    top_inset: f32,
    progress: f32,
    on_backdrop_click: Option<Message>,
) -> Element<'a, Message, AppTheme> {
    let progress = progress.clamp(0.0, 1.0);
    let backdrop_color = Color {
        a: progress * BACKDROP_ALPHA,
        ..Color::BLACK
    };
    let backdrop = container(Space::new())
        .width(Fill)
        .height(Fill)
        .style(move |_theme: &AppTheme| container::Style::default().background(backdrop_color));

    let backdrop: Element<'a, Message, AppTheme> = match on_backdrop_click {
        Some(msg) => mouse_area(backdrop).on_press(msg).into(),
        None => backdrop.into(),
    };

    let animated_inset = top_inset + (1.0 - progress) * SLIDE_OFFSET_PX;
    // `opaque` on the sheet body absorbs click + hover events so they
    // don't fall through to the dismiss-mouse_area on the backdrop. The
    // top spacer is *not* opaque so clicks above the sheet still dismiss.
    let sheet = opaque(container(content).width(Fill).height(Fill));
    let offset_content = column![
        Space::new()
            .width(Fill)
            .height(Length::Fixed(animated_inset)),
        sheet,
    ]
    .width(Fill)
    .height(Fill);

    // Outer `opaque` blocks hover/clicks from reaching widgets beneath the
    // sheet so they don't light up while the sheet is up.
    opaque(stack![backdrop, offset_content].width(Fill).height(Fill))
}
