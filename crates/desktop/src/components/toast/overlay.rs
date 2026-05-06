//! `ToastOverlay` — the overlay element rendered by `Manager::overlay`.
//!
//! Owns the per-frame state machine: appearing → visible → dismissing →
//! dropped. Refreshes visuals every `RedrawRequested` tick.

use std::{cell::Cell, rc::Rc, time::Duration};

use iced::{
    Alignment, Element, Event, Length, Point, Rectangle, Size, Vector,
    advanced::{
        Layout, Shell,
        layout::{self, Limits, Node},
        mouse::{self, Cursor},
        overlay,
        renderer::{self},
        widget::Tree,
    },
    window,
};

use crate::theme::AppTheme;

use super::{
    ANIMATION_TICK_MS, FADE_IN_MS, FADE_OUT_MS, OVERLAY_SIDE_PAD, OVERLAY_TOP_PAD, TIMEOUT,
    visuals::{ToastTimer, ToastVisuals, refresh_visuals},
};

pub(super) struct ToastOverlay<'a, 'b, Message> {
    pub(super) position: Point,
    pub(super) viewport: Rectangle,
    pub(super) toasts: &'b mut [Element<'a, Message, AppTheme>],
    pub(super) trees: &'b mut [Tree],
    pub(super) timers: &'b mut [Option<ToastTimer>],
    pub(super) cells: &'b [Rc<Cell<ToastVisuals>>],
    pub(super) on_close: &'b dyn Fn(usize) -> Message,
}

impl<Message> overlay::Overlay<Message, AppTheme, iced::Renderer> for ToastOverlay<'_, '_, Message>
where
    Message: Clone,
{
    fn layout(&mut self, renderer: &iced::Renderer, bounds: Size) -> Node {
        let limits = Limits::new(Size::ZERO, bounds);

        let padding = iced::Padding {
            top: OVERLAY_TOP_PAD,
            right: OVERLAY_SIDE_PAD,
            bottom: 0.0,
            left: OVERLAY_SIDE_PAD,
        };

        layout::flex::resolve(
            layout::flex::Axis::Vertical,
            renderer,
            &limits,
            Length::Fill,
            Length::Fill,
            padding,
            10.0,
            Alignment::End,
            self.toasts,
            self.trees,
        )
        .translate(Vector::new(self.position.x, self.position.y))
    }

    fn update(
        &mut self,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
        shell: &mut Shell<'_, Message>,
    ) {
        if let Event::Window(window::Event::RedrawRequested(now)) = event {
            // Per-toast state machine: appearing → visible → dismissing →
            // dropped. Alpha and progress are refreshed every tick.
            let fade_out_d = Duration::from_millis(FADE_OUT_MS);
            let cursor_pos = cursor.position();
            let mut child_layouts = layout.children();

            for ((index, slot), cell) in self.timers.iter_mut().enumerate().zip(self.cells.iter()) {
                let Some(child_layout) = child_layouts.next() else {
                    continue;
                };
                let Some(timer) = slot.as_mut() else {
                    continue;
                };

                // Hover detection: if the cursor is inside this toast's
                // bounds, pin to fully visible. Cancel any in-flight dismissal
                // and slide `created` forward so the timer resumes from full
                // when the cursor leaves.
                let cursor_over = cursor_pos
                    .map(|p| child_layout.bounds().contains(p))
                    .unwrap_or(false);
                timer.hovered = cursor_over;
                if cursor_over {
                    timer.dismissing = None;
                    // Past the fade-in so we don't re-fade-in on un-hover.
                    timer.created = *now - Duration::from_millis(FADE_IN_MS);
                }

                // Phase 1: kick off auto-dismiss once visible timer expires.
                if timer.dismissing.is_none() && timer.created.elapsed() >= TIMEOUT {
                    timer.dismissing = Some(*now);
                }

                // Phase 2: publish on_close once fade-out completes. The
                // toast drops from `App.toasts` next frame, so no need to
                // refresh the cell — its `Element` won't be drawn again.
                if let Some(start) = timer.dismissing
                    && now.saturating_duration_since(start) >= fade_out_d
                {
                    *slot = None;
                    shell.publish((self.on_close)(index));
                    continue;
                }

                cell.set(refresh_visuals(timer, *now));

                shell.request_redraw_at(*now + Duration::from_millis(ANIMATION_TICK_MS));
            }
        }

        let viewport = layout.bounds();

        for (((child, state), child_layout), timer_slot) in self
            .toasts
            .iter_mut()
            .zip(self.trees.iter_mut())
            .zip(layout.children())
            .zip(self.timers.iter_mut())
        {
            let mut local_messages = Vec::new();
            let mut local_shell = Shell::new(&mut local_messages);

            child.as_widget_mut().update(
                state,
                event,
                child_layout,
                cursor,
                renderer,
                &mut local_shell,
                &viewport,
            );

            // Any local message (e.g. close button click) is an explicit
            // dismissal — drop the timer so the post-update sweep doesn't
            // re-publish on_close.
            if !local_shell.is_empty() {
                *timer_slot = None;
            }
            shell.merge(local_shell, std::convert::identity);
        }
    }

    fn draw(
        &self,
        renderer: &mut iced::Renderer,
        theme: &AppTheme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
    ) {
        let viewport = layout.bounds();
        for ((child, tree), child_layout) in self
            .toasts
            .iter()
            .zip(self.trees.iter())
            .zip(layout.children())
        {
            child.as_widget().draw(
                tree,
                renderer,
                theme,
                style,
                child_layout,
                cursor,
                &viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.toasts
            .iter()
            .zip(self.trees.iter())
            .zip(layout.children())
            .map(|((child, state), child_layout)| {
                child.as_widget().mouse_interaction(
                    state,
                    child_layout,
                    cursor,
                    &self.viewport,
                    renderer,
                )
            })
            .max()
            .unwrap_or_default()
    }
}
