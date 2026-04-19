//! TOTP code display with a circular countdown ring.
//!
//! Rendered when a login cipher has a non-empty `totp` field. The parent
//! (vault view) subscribes to a 1 Hz tick while a TOTP is visible — see
//! `VaultView::has_totp_selected` — so `view()` re-runs every second and
//! this helper recomputes the code + seconds-remaining on each render.
//!
//! The copy button emits the caller's `on_copy` message; the recipient is
//! expected to recompute the code from the secret at copy time so the
//! clipboard always holds a value that's still valid.

use bitwarden_vault::{TotpResponse, generate_totp};
use iced::{
    Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer,
    advanced::mouse,
    alignment as iced_alignment,
    widget::{
        canvas::{
            self, Fill as CanvasFill, Frame, Geometry, LineCap, Path, Stroke, Text as CanvasText,
        },
        column, container, row, text,
    },
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
/// Flip the arc + countdown-text color to red when this many seconds or
/// fewer remain in the current TOTP period.
const URGENT_THRESHOLD: u32 = 5;
const URGENT_COLOR: Color = Color::from_rgb(0.91, 0.28, 0.28);

/// Format a 6-digit TOTP as "123 456" (or leave untouched if the length
/// doesn't match the standard — e.g. Steam codes).
fn format_code(code: &str) -> String {
    if code.len() == 6 && code.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {}", &code[..3], &code[3..])
    } else {
        code.to_owned()
    }
}

/// Build the TOTP field: label + formatted code + countdown ring + copy.
/// Falls back to a "Invalid code" display when the secret can't be
/// parsed by the SDK (bad base32, malformed otpauth URI, etc.) so the
/// user sees that the seed was stored but isn't currently usable.
pub fn view<'a, Message: 'a + Clone>(
    secret: &str,
    on_copy: Message,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme> {
    let label = text(fl!("detail-field-totp"))
        .size(12)
        .color(colors.text_muted);

    let (value_row, ring): (
        Element<'a, Message, AppTheme>,
        Element<'a, Message, AppTheme>,
    ) = match generate_totp(secret.to_owned(), None) {
        Ok(response) => {
            let seconds_remaining = seconds_remaining(&response);
            let urgent = seconds_remaining <= URGENT_THRESHOLD;
            let code_color = if urgent {
                URGENT_COLOR
            } else {
                colors.text_primary
            };

            let code = text(format_code(&response.code))
                .size(18)
                .color(code_color)
                .font(crate::APP_FONT_BOLD);

            let ring_el: Element<'a, Message, AppTheme> = iced::widget::canvas(CountdownRing {
                period: response.period,
                seconds_remaining,
                stroke: RING_STROKE,
                active: if urgent { URGENT_COLOR } else { colors.accent },
                track: colors.border,
                text_color: if urgent {
                    URGENT_COLOR
                } else {
                    colors.text_primary
                },
                background: colors.background,
            })
            .width(Length::Fixed(RING_SIZE))
            .height(Length::Fixed(RING_SIZE))
            .into();

            (code.into(), ring_el)
        }
        Err(err) => {
            tracing::debug!(%err, "generate_totp failed; showing fallback");
            let fallback = text(fl!("detail-totp-invalid"))
                .size(14)
                .color(colors.text_muted);
            let empty_ring: Element<'a, Message, AppTheme> = container(iced::widget::Space::new())
                .width(Length::Fixed(RING_SIZE))
                .height(Length::Fixed(RING_SIZE))
                .into();
            (fallback.into(), empty_ring)
        }
    };

    let copy_btn = icon_button(icons::BWI_COPY, on_copy, colors);

    row![
        column![label, value_row].spacing(2).width(Fill),
        ring,
        copy_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

/// Derive the seconds-remaining in the current TOTP period from the
/// system wall clock. TOTP alignment is against UTC seconds so no
/// timestamp parameter is needed.
fn seconds_remaining(response: &TotpResponse) -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let elapsed = now % u64::from(response.period);
    u32::try_from(u64::from(response.period) - elapsed).unwrap_or(response.period)
}

// ── Countdown ring ─────────────────────────────────────────────────────────

struct CountdownRing {
    period: u32,
    seconds_remaining: u32,
    stroke: f32,
    active: Color,
    track: Color,
    text_color: Color,
    background: Color,
}

impl<Message> canvas::Program<Message, AppTheme> for CountdownRing {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &AppTheme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Paint the surface underneath first — without this the tiny-skia
        // stroker occasionally leaves a stray pixel opposite the arc
        // endpoints; filling the full bounds absorbs it. Keep this color
        // matched to whatever parent container sits behind the canvas.
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            CanvasFill::from(self.background),
        );

        let center = Point::new((bounds.width / 2.0).round(), (bounds.height / 2.0).round());
        // We remove one from the radius to avoid clipping at the outer edge.
        let radius = (bounds.width.min(bounds.height) / 2.0 - self.stroke / 2.0 - 1.0).round();

        // Background track — full circle.
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_width(self.stroke)
                .with_color(self.track)
                .with_line_cap(LineCap::Butt),
        );

        // Foreground arc — starts at 12 o'clock, sweeps clockwise,
        // length proportional to time remaining.
        let fraction = (self.seconds_remaining as f32 / self.period.max(1) as f32).clamp(0.0, 1.0);
        if fraction > 0.0 {
            let start = -std::f32::consts::FRAC_PI_2;
            let end = start + fraction * std::f32::consts::TAU;
            let arc = Path::new(|p| {
                p.arc(canvas::path::Arc {
                    center,
                    radius,
                    start_angle: iced::Radians(start),
                    end_angle: iced::Radians(end),
                });
            });
            frame.stroke(
                &arc,
                Stroke::default()
                    .with_width(self.stroke)
                    .with_color(self.active)
                    .with_line_cap(LineCap::Butt),
            );
        }

        // Seconds-remaining number centered inside the ring.
        frame.fill_text(CanvasText {
            content: format!("{}", self.seconds_remaining),
            position: center,
            color: self.text_color,
            size: 12.into(),
            align_x: iced::advanced::text::Alignment::Center,
            align_y: iced_alignment::Vertical::Center,
            font: crate::APP_FONT_BOLD,
            ..Default::default()
        });

        vec![frame.into_geometry()]
    }
}
