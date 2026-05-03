//! Account → Fingerprint phrase modal.
//!
//! Single-shot dialog: opened from the menu with a precomputed phrase, closed
//! via the Close button or backdrop click. The "Learn more" link opens
//! Bitwarden's fingerprint help page in the default browser. State lives on
//! `App` rather than as its own `View` because the modal carries one string
//! and emits three App-level signals — full MVU plumbing would be boilerplate.
//!
//! Pure helper, not a `View`: `modal_view` is generic over a message type
//! `M` and takes its three message instances by parameter, so this module
//! never imports `crate::app::Message`. The caller (App) constructs the
//! concrete `Message` values and passes them in.

use iced::{
    Alignment, Border, Color, Element, Fill, Length, Padding, Shadow, Vector,
    widget::{Space, column, container, row, text, text::Wrapping},
};

use crate::{
    components::{FadeInOut, buttons, icons, modal},
    fl,
    services::clipboard,
    theme::AppTheme,
};

const LEARN_MORE_URL: &str = "https://bitwarden.com/help/fingerprint-phrase/";

/// Info-icon ring diameter in px. The icon glyph is centered inside.
const RING_DIAMETER: f32 = 48.0;

#[derive(Default)]
pub struct FingerprintModal {
    pub(super) fade: FadeInOut,
    phrase: String,
}

impl FingerprintModal {
    pub fn open_with(&mut self, phrase: String) {
        self.phrase = phrase;
        self.fade.open();
    }

    pub fn close(&mut self) {
        self.fade.close();
    }

    pub fn phrase(&self) -> &str {
        &self.phrase
    }
}

/// Returns `None` while fully closed so App can drop the slot from its
/// overlay stack rather than rendering an invisible layer.
///
/// `on_copy` / `on_close` / `on_learn_more` are concrete message instances
/// the caller wants emitted on the corresponding action. Generic over `M` so
/// this module doesn't depend on `App::Message`.
pub fn modal_view<'a, M: Clone + 'a>(
    ctx: &crate::app::RenderCtx<'a>,
    state: &'a FingerprintModal,
    on_copy: M,
    on_close: M,
    on_learn_more: M,
) -> Option<Element<'a, M, AppTheme>> {
    let progress = state.fade.progress_if_visible()?;

    let icon_ring =
        container(icons::INFO_CIRCLE_FILL.render::<M, AppTheme>(28.0, ctx.colors.accent))
            .width(Length::Fixed(RING_DIAMETER))
            .height(Length::Fixed(RING_DIAMETER))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(Color {
                        a: 0.12,
                        ..theme.colors.accent
                    })
                    .border(Border::default().rounded(RING_DIAMETER / 2.0))
                    .shadow(Shadow {
                        color: Color {
                            a: 0.18,
                            ..theme.colors.accent
                        },
                        offset: Vector::new(0.0, 2.0),
                        blur_radius: 12.0,
                    })
            });

    let title = text(fl!("menu-fingerprint-title"))
        .size(16)
        .color(ctx.colors.text_primary)
        .font(crate::APP_FONT_BOLD);

    let phrase_row = row![
        text(state.phrase.as_str())
            .size(14)
            .color(ctx.colors.text_primary)
            .wrapping(Wrapping::None),
        buttons::icon_button(icons::BWI_COPY, on_copy, ctx.colors),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let learn_more = buttons::primary(
        row![
            text(fl!("menu-fingerprint-learn-more")).size(14),
            icons::BWI_EXTERNAL_LINK.render::<M, AppTheme>(12.0, ctx.colors.card_bg),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(on_learn_more)
    .padding(Padding::from([10, 20]))
    .width(Length::Fill);

    let close_button = buttons::secondary(
        text(fl!("menu-fingerprint-close"))
            .size(14)
            .color(ctx.colors.accent),
    )
    .on_press(on_close.clone())
    .padding(Padding::from([10, 20]))
    .width(Length::Fill);

    let body = column![
        container(icon_ring).width(Fill).align_x(Alignment::Center),
        Space::new().height(Length::Fixed(8.0)),
        column![title, phrase_row]
            .spacing(8)
            .align_x(Alignment::Center)
            .width(Fill),
        Space::new().height(Length::Fixed(8.0)),
        learn_more,
        close_button,
    ]
    .spacing(10)
    .padding(Padding {
        top: 24.0,
        right: 24.0,
        bottom: 20.0,
        left: 24.0,
    })
    .align_x(Alignment::Center)
    .width(Fill);

    Some(modal::dialog(
        440.0,
        None,
        |c| c.card_bg,
        progress,
        body,
        on_close,
    ))
}

/// Open the fingerprint help page in the default browser.
pub fn open_learn_more() {
    clipboard::launch_url(LEARN_MORE_URL);
}
