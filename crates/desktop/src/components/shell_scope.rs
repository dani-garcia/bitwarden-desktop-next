//! Wraps a child [`Element`] so that its event-time `update` runs against a
//! private [`Shell`] rather than the parent's, then merges the result back
//! up. This isolates the child from capture-flag leaks that occur when an
//! earlier sibling widget called `shell.capture_event()`.
//!
//! **Why this exists.** [`iced::widget::Stack`] short-circuits its
//! between-children iteration on `shell.is_event_captured()`
//! (widget/src/stack.rs in the pinned iced rev). The shell is *shared* across
//! the full event-handling pass — siblings of one stack share a shell with
//! siblings of another stack. So a capture set inside one widget's
//! [`Stack`] persists when iced moves on to the next sibling subtree, and
//! that next subtree's own [`Stack`] then bails out before reaching the
//! widget that should have run.
//!
//! Concrete repro: two `pick_list`-backed
//! [`field_frame`](crate::components::inputs::field_frame)s as siblings in a
//! column. Open the second, click the first → the first's picker captures
//! (opens), and the second's stack short-circuits because the shell is
//! already captured, so the second's picker never runs the close branch and
//! both menus end up open.
//!
//! Wrapping each [`field_frame`] result in [`ShellScope`] gives every field
//! its own clean shell — sibling captures don't leak in. Anything the inner
//! subtree wants to report (messages, redraw requests, clipboard ops,
//! capture status) flows back through [`Shell::merge`].

use iced::{
    Element, Event, Length, Rectangle, Renderer, Size, Vector,
    advanced::{
        Layout, Shell, Widget,
        layout, mouse, overlay,
        renderer,
        widget::{Operation, Tree, tree},
    },
};

use crate::theme::AppTheme;

pub struct ShellScope<'a, Message> {
    inner: Element<'a, Message, AppTheme>,
}

impl<'a, Message> ShellScope<'a, Message> {
    pub fn new(inner: impl Into<Element<'a, Message, AppTheme>>) -> Self {
        Self {
            inner: inner.into(),
        }
    }
}

impl<Message> Widget<Message, AppTheme, Renderer> for ShellScope<'_, Message> {
    fn size(&self) -> Size<Length> {
        self.inner.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.inner.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.inner.as_widget_mut().layout(tree, renderer, limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &AppTheme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inner
            .as_widget()
            .draw(tree, renderer, theme, style, layout, cursor, viewport);
    }

    fn tag(&self) -> tree::Tag {
        self.inner.as_widget().tag()
    }

    fn state(&self) -> tree::State {
        self.inner.as_widget().state()
    }

    fn children(&self) -> Vec<Tree> {
        self.inner.as_widget().children()
    }

    fn diff(&self, tree: &mut Tree) {
        self.inner.as_widget().diff(tree);
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.inner
            .as_widget_mut()
            .operate(tree, layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Run the inner widget against a private shell so its
        // `is_event_captured()` view starts at `Ignored`, regardless of what
        // any earlier sibling has done with the parent shell. After the
        // inner widget finishes, fold its outputs (messages, capture status,
        // redraw, layout/widget invalidation, clipboard, input method) back
        // into the parent via `Shell::merge`.
        let mut inner_messages: Vec<Message> = Vec::new();
        let mut inner_shell = Shell::new(&mut inner_messages);
        self.inner.as_widget_mut().update(
            tree,
            event,
            layout,
            cursor,
            renderer,
            &mut inner_shell,
            viewport,
        );
        shell.merge(inner_shell, |m| m);
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.inner
            .as_widget()
            .mouse_interaction(tree, layout, cursor, viewport, renderer)
    }

    fn overlay<'a>(
        &'a mut self,
        tree: &'a mut Tree,
        layout: Layout<'a>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'a, Message, AppTheme, Renderer>> {
        self.inner
            .as_widget_mut()
            .overlay(tree, layout, renderer, viewport, translation)
    }
}

impl<'a, Message> From<ShellScope<'a, Message>> for Element<'a, Message, AppTheme>
where
    Message: 'a,
{
    fn from(value: ShellScope<'a, Message>) -> Self {
        Self::new(value)
    }
}
