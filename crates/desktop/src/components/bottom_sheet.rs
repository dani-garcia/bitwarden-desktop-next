use iced::{
    Color, Element, Fill, Length,
    widget::{Space, column, container, mouse_area, opaque, stack},
};

use crate::theme::AppTheme;

/// Backdrop alpha at full open. Modulated by `progress` so the scrim
/// fades in/out alongside the sheet — same trick as `modal::view`.
const BACKDROP_ALPHA: f32 = 0.45;

/// How far below its rest position the sheet starts during the open
/// transition. Faked via animated top-padding inside the offset column —
/// iced has no transform widget. 32 px reads as a noticeable slide
/// without making the sheet "leave" the screen entirely on outro.
const SLIDE_OFFSET_PX: f32 = 32.0;

/// Wrap `content` as a sheet that fills the available area starting
/// `top_inset` pixels below the top. A full-height darkened backdrop
/// sits behind the sheet; clicks on the exposed backdrop fire
/// `on_backdrop_click` if provided (z-order means the sheet still
/// absorbs clicks on itself).
///
/// `progress` is the open-animation phase, `0.0` (fully closed — caller
/// shouldn't render at all) → `1.0` (fully open). Drives backdrop alpha
/// and slide-up position. Pass `1.0` for non-animated callers.
///
/// The caller's content is responsible for its own background and
/// rounded corners — this wrapper provides only positioning and scrim.
/// (iced doesn't clip children to parent border radius, so any rounded
/// shape has to come from the content itself.)
///
/// Generic over the message type so it can host any sub-view's element.
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

    // Slide origin: at progress=0 the sheet sits SLIDE_OFFSET_PX below
    // its rest position, sliding up to 0 as it opens. Implemented by
    // adding the offset to the sheet's top inset.
    let animated_inset = top_inset + (1.0 - progress) * SLIDE_OFFSET_PX;
    // `opaque` on the sheet body absorbs click + hover events so they
    // don't fall through to the dismiss-mouse_area on the backdrop. The
    // top spacer is *not* opaque so clicks above the sheet still dismiss.
    let sheet = opaque(container(content).width(Fill).height(Fill));
    let offset_content = column![
        Space::new().width(Fill).height(Length::Fixed(animated_inset)),
        sheet,
    ]
    .width(Fill)
    .height(Fill);

    // Outer `opaque` mirrors the modal pattern: the entire sheet overlay
    // is opaque to widgets below it (so hover/clicks on the page beneath
    // can't light up while the sheet is up).
    opaque(
        stack![backdrop, offset_content]
            .width(Fill)
            .height(Fill),
    )
}
