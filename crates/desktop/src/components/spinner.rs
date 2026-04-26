//! Self-animating indeterminate spinner.
//!
//! Drives its own redraws by intercepting [`window::Event::RedrawRequested`]
//! and calling `shell.request_redraw()` — no app-level subscription needed.

use std::time::Instant;

use iced::{
    Background, Border, Color, Element, Event, Length, Rectangle, Renderer, Shadow, Size,
    advanced::{
        Layout, Shell, Widget,
        layout::{Limits, Node},
        mouse::{self, Cursor},
        renderer::{self, Quad},
        widget::{self, Tree},
    },
    window,
};

use crate::theme::AppTheme;

const N_DOTS: usize = 6;
/// Full revolutions per second.
const REV_PER_SEC: f32 = 1.0;

pub fn spinner<Message>(size: f32, color: Color) -> Element<'static, Message, AppTheme>
where
    Message: 'static,
{
    Element::new(Spinner {
        size,
        color,
        start: Instant::now(),
    })
}

struct Spinner {
    size: f32,
    color: Color,
    start: Instant,
}

impl<Message> Widget<Message, AppTheme, Renderer> for Spinner {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, _limits: &Limits) -> Node {
        Node::new(Size::new(self.size, self.size))
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        _layout: Layout<'_>,
        _cursor: Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        if let Event::Window(window::Event::RedrawRequested(_)) = event {
            shell.request_redraw();
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut Renderer,
        _theme: &AppTheme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;

        let bounds = layout.bounds();
        let dot_size = self.size / 5.0;
        let ring_radius = self.size / 2.0 - dot_size / 2.0;
        let cx = bounds.x + self.size / 2.0;
        let cy = bounds.y + self.size / 2.0;

        let rotation = self.start.elapsed().as_secs_f32() * REV_PER_SEC * std::f32::consts::TAU;

        for i in 0..N_DOTS {
            // i = 0 is the "head" (brightest); subsequent dots trail
            // counter-clockwise with progressively lower alpha.
            let angle = rotation - (i as f32 / N_DOTS as f32) * std::f32::consts::TAU;
            let alpha = 1.0 - (i as f32 / N_DOTS as f32) * 0.85;
            let x = cx + angle.cos() * ring_radius - dot_size / 2.0;
            let y = cy + angle.sin() * ring_radius - dot_size / 2.0;

            renderer.fill_quad(
                Quad {
                    bounds: Rectangle {
                        x,
                        y,
                        width: dot_size,
                        height: dot_size,
                    },
                    border: Border::default().rounded(dot_size / 2.0),
                    shadow: Shadow::default(),
                    snap: false,
                },
                Background::Color(self.color.scale_alpha(alpha)),
            );
        }
    }

    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::stateless()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::None
    }

    fn mouse_interaction(
        &self,
        _tree: &Tree,
        _layout: Layout<'_>,
        _cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        mouse::Interaction::None
    }
}
