//! TOTP code display with a self-animating circular countdown ring.
//!
//! The code + ring live inside a single [`CountdownField`] widget that
//! intercepts [`window::Event::RedrawRequested`] and schedules its own next
//! tick via `shell.request_redraw_at`, aligned to the next whole UNIX
//! second. No app-level `time::every` subscription is involved, so the rest
//! of the window doesn't rebuild and repaint once per second just to
//! advance the countdown.
//!
//! The code text is rendered via canvas so it can't be selected; the
//! widget hit-tests clicks against the code-text region (everything except
//! the ring on the right edge) and emits the same `on_copy` message the
//! copy button does. The caller is expected to recompute the TOTP from the
//! secret at copy time so the clipboard always holds a value that's still
//! valid.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bitwarden_vault::{TotpResponse, generate_totp};
use iced::{
    Alignment, Color, Element, Event, Fill, Length, Point, Rectangle, Renderer, Size, Vector,
    advanced::{
        Layout, Renderer as _, Shell, Widget,
        graphics::geometry::Renderer as _,
        layout::{Limits, Node},
        mouse::{self, Cursor},
        renderer, text as iced_text,
        widget::{self, Tree},
    },
    alignment as iced_alignment,
    widget::{
        canvas::{Fill as CanvasFill, Frame, LineCap, Path, Stroke, Text as CanvasText, path::Arc},
        column, row, text,
    },
    window,
};

use crate::{
    components::icons,
    fl,
    theme::{AppColors, AppTheme},
    views::vault::widgets::field_helpers::icon_button,
};

/// Diameter of the countdown ring in logical pixels.
const RING_SIZE: f32 = 30.0;
const RING_STROKE: f32 = 3.0;
/// Flip the arc + code colour to red when this many seconds or fewer
/// remain in the current TOTP period.
const URGENT_THRESHOLD: u32 = 5;
const URGENT_COLOR: Color = Color::from_rgb(0.91, 0.28, 0.28);
const CODE_FONT_SIZE: f32 = 18.0;
/// Extra slack on top of the next-whole-second instant so we tick *past*
/// the boundary and read the new integer second, not the old one.
const BOUNDARY_SLACK: Duration = Duration::from_millis(10);

/// Format a 6-digit TOTP as "123 456" (or leave untouched if the length
/// doesn't match the standard — e.g. Steam codes).
fn format_code(code: &str) -> String {
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {}", &code[..3], &code[3..])
    } else {
        code.to_owned()
    }
}

/// Build the TOTP field: label + self-animating code/ring + copy button.
pub fn view<'a, Message: 'a + Clone>(
    secret: &'a str,
    on_copy: Message,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme> {
    let label = text(fl!("detail-field-totp"))
        .size(12)
        .color(colors.text_muted);

    // Canvas text isn't selectable; the widget itself handles click-to-copy
    // on the code region (left of the ring) and renders a pointer cursor
    // over that region. The ring is non-interactive.
    let field: Element<'a, Message, AppTheme> = Element::new(CountdownField {
        secret,
        colors,
        on_copy: on_copy.clone(),
    });
    let copy_btn = icon_button(icons::BWI_COPY, on_copy, colors);

    // Label sits on its own row; the field (code + ring) and copy button
    // share the row below so the copy button aligns with the ring rather
    // than the taller label-plus-field column's midpoint.
    column![
        label,
        row![field, copy_btn]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Fill),
    ]
    .spacing(2)
    .into()
}

// ── Self-driving widget ────────────────────────────────────────────────────

struct CountdownField<'a, Message> {
    secret: &'a str,
    colors: &'a AppColors,
    on_copy: Message,
}

impl<Message> CountdownField<'_, Message> {
    /// Recompute code + seconds-remaining from the current system clock.
    /// `None` when the secret can't be parsed by the SDK — we'll render the
    /// "Invalid code" fallback and skip scheduling further redraws.
    fn current(&self) -> Option<(TotpResponse, u32, bool)> {
        let resp = generate_totp(self.secret.to_owned(), None).ok()?;
        let rem = seconds_remaining_from(resp.period);
        Some((resp, rem, rem <= URGENT_THRESHOLD))
    }
}

/// Hit area for click-to-copy: should be close enough to the code text bounds,
/// without having to calculate the text width.
fn code_hit_bounds(bounds: Rectangle) -> Rectangle {
    Rectangle {
        x: bounds.x,
        y: bounds.y,
        width: CODE_FONT_SIZE * 5.0, // Good enough for 8 chars at least.
        height: bounds.height,
    }
}

/// Seconds remaining in the current TOTP period (UTC-aligned, per RFC 6238).
fn seconds_remaining_from(period: u32) -> u32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let elapsed = now % u64::from(period);
    u32::try_from(u64::from(period) - elapsed).unwrap_or(period)
}

/// Wall-clock delay until the next whole UNIX second, plus a small slack
/// so the scheduled redraw reads the post-boundary integer second.
fn delay_to_next_second() -> Duration {
    let subsec = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    Duration::from_nanos(1_000_000_000 - subsec) + BOUNDARY_SLACK
}

impl<Message> Widget<Message, AppTheme, Renderer> for CountdownField<'_, Message>
where
    Message: Clone,
{
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(RING_SIZE))
    }

    fn layout(&mut self, _tree: &mut Tree, _renderer: &Renderer, limits: &Limits) -> Node {
        let size = limits.resolve(
            Length::Fill,
            Length::Fixed(RING_SIZE),
            Size::new(0.0, RING_SIZE),
        );
        Node::new(size)
    }

    fn update(
        &mut self,
        _tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: Cursor,
        _renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) {
        match event {
            Event::Window(window::Event::RedrawRequested(now)) if self.current().is_some() => {
                shell.request_redraw_at(*now + delay_to_next_second());
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(code_hit_bounds(layout.bounds())) =>
            {
                shell.publish(self.on_copy.clone());
            }
            _ => {}
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
        let bounds = layout.bounds();
        let mut frame = Frame::new(renderer, bounds.size());

        // Fill the whole widget bounds every frame. Two reasons:
        //   1. Iced doesn't repaint the area under a self-driving widget
        //      between frames, so leftover anti-aliased glyph pixels from
        //      the previous frame would show through where the new glyphs
        //      don't fully cover them (e.g. on an urgent colour flip the
        //      old-colour AA fringes linger against the new-colour body).
        //   2. Same role the original ring-only fill played: absorb the
        //      tiny-skia stroker's occasional stray pixel opposite the arc
        //      endpoints.
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            CanvasFill::from(self.colors.background),
        );

        match self.current() {
            Some((resp, rem, urgent)) => {
                let code_color = if urgent {
                    URGENT_COLOR
                } else {
                    self.colors.text_primary
                };
                let arc_color = if urgent {
                    URGENT_COLOR
                } else {
                    self.colors.accent
                };

                // Code text — left-aligned, centred vertically.
                frame.fill_text(CanvasText {
                    content: format_code(&resp.code),
                    position: Point::new(0.0, bounds.height / 2.0),
                    color: code_color,
                    size: CODE_FONT_SIZE.into(),
                    align_x: iced_text::Alignment::Left,
                    align_y: iced_alignment::Vertical::Center,
                    font: crate::APP_FONT_BOLD,
                    ..Default::default()
                });

                let center = Point::new(
                    (bounds.width - RING_SIZE / 2.0).round(),
                    (bounds.height / 2.0).round(),
                );
                let radius = (RING_SIZE / 2.0 - RING_STROKE / 2.0 - 1.0).round();

                // Background track — full circle.
                frame.stroke(
                    &Path::circle(center, radius),
                    Stroke::default()
                        .with_width(RING_STROKE)
                        .with_color(self.colors.border)
                        .with_line_cap(LineCap::Butt),
                );

                // Foreground arc — starts at 12 o'clock, sweeps clockwise,
                // length proportional to time remaining.
                let fraction = (rem as f32 / resp.period.max(1) as f32).clamp(0.0, 1.0);
                if fraction > 0.0 {
                    let start = -std::f32::consts::FRAC_PI_2;
                    let end = start + fraction * std::f32::consts::TAU;
                    let arc_path = Path::new(|p| {
                        p.arc(Arc {
                            center,
                            radius,
                            start_angle: iced::Radians(start),
                            end_angle: iced::Radians(end),
                        });
                    });
                    frame.stroke(
                        &arc_path,
                        Stroke::default()
                            .with_width(RING_STROKE)
                            .with_color(arc_color)
                            .with_line_cap(LineCap::Butt),
                    );
                }

                // Seconds-remaining number centred inside the ring.
                frame.fill_text(CanvasText {
                    content: format!("{rem}"),
                    position: center,
                    color: code_color,
                    size: 12.into(),
                    align_x: iced::advanced::text::Alignment::Center,
                    align_y: iced_alignment::Vertical::Center,
                    font: crate::APP_FONT_BOLD,
                    ..Default::default()
                });
            }
            None => {
                // Fallback — secret couldn't be parsed (bad base32, malformed
                // otpauth URI, …). No redraw is scheduled in `update` for
                // this branch so the widget stays still.
                frame.fill_text(CanvasText {
                    content: fl!("detail-totp-invalid"),
                    position: Point::new(0.0, bounds.height / 2.0),
                    color: self.colors.text_muted,
                    size: 14.into(),
                    align_x: iced::advanced::text::Alignment::Left,
                    align_y: iced_alignment::Vertical::Center,
                    font: iced::Font::DEFAULT,
                    ..Default::default()
                });
            }
        }

        renderer.with_translation(Vector::new(bounds.x, bounds.y), |r| {
            r.draw_geometry(frame.into_geometry());
        });
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
        layout: Layout<'_>,
        cursor: Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> mouse::Interaction {
        if cursor.is_over(code_hit_bounds(layout.bounds())) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::None
        }
    }
}
