use iced::{
    Color, Element, Fill, Length,
    widget::{Space, column, container, mouse_area, stack},
};

use crate::theme::AppTheme;

/// Darkening over the strip above the sheet. Acts as a scrim that the
/// sheet is the active surface; mixed against whatever the host renders
/// behind it.
const BACKDROP: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.45);

/// Wrap `content` as a sheet that fills the available area starting
/// `top_inset` pixels below the top. A full-height darkened backdrop
/// sits behind the sheet; clicks on the exposed backdrop fire
/// `on_backdrop_click` if provided (z-order means the sheet still
/// absorbs clicks on itself).
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
    on_backdrop_click: Option<Message>,
) -> Element<'a, Message, AppTheme> {
    let backdrop = container(Space::new())
        .width(Fill)
        .height(Fill)
        .style(|_theme: &AppTheme| container::Style::default().background(BACKDROP));

    let backdrop: Element<'a, Message, AppTheme> = match on_backdrop_click {
        Some(msg) => mouse_area(backdrop).on_press(msg).into(),
        None => backdrop.into(),
    };

    let offset_content = column![
        Space::new().width(Fill).height(Length::Fixed(top_inset)),
        container(content).width(Fill).height(Fill),
    ]
    .width(Fill)
    .height(Fill);

    stack![backdrop, offset_content]
        .width(Fill)
        .height(Fill)
        .into()
}
