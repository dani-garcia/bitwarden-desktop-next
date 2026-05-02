//! Account → Fingerprint phrase modal.
//!
//! Single-shot dialog: opened from the menu with a precomputed phrase, closed
//! via the Close button or backdrop click. The "Learn more" link opens
//! Bitwarden's fingerprint help page in the default browser. State lives on
//! `App` rather than as its own `View` because the modal carries one string
//! and emits two App-level signals — full MVU plumbing would be boilerplate.

use iced::{
    Alignment, Border, Color, Element, Fill, Length, Padding, Shadow, Vector,
    widget::{Space, column, container, row, text, text::Wrapping},
};

use crate::{
    app::{Message, SystemMessage},
    components::{FadeInOut, buttons, icons, modal},
    fl,
    services::clipboard,
    theme::{AppColors, AppTheme},
};

const LEARN_MORE_URL: &str = "https://bitwarden.com/help/fingerprint-phrase/";

/// Info-icon ring diameter in px. The icon glyph is centered inside.
const RING_DIAMETER: f32 = 48.0;

#[derive(Default)]
pub struct FingerprintModal {
    pub fade: FadeInOut,
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
pub fn modal_view<'a>(
    state: &'a FingerprintModal,
    colors: &'a AppColors,
) -> Option<Element<'a, Message, AppTheme>> {
    let progress = state.fade.progress_if_visible()?;

    let icon_ring =
        container(icons::INFO_CIRCLE_FILL.render::<Message, AppTheme>(28.0, colors.accent))
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
        .color(colors.text_primary)
        .font(crate::APP_FONT_BOLD);

    let phrase_row = row![
        text(state.phrase.as_str())
            .size(14)
            .color(colors.text_primary)
            .wrapping(Wrapping::None),
        buttons::icon_button(
            icons::BWI_COPY,
            Message::System(SystemMessage::CopyFingerprint),
            colors,
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let learn_more = buttons::primary(
        row![
            text(fl!("menu-fingerprint-learn-more")).size(14),
            icons::BWI_EXTERNAL_LINK.render::<Message, AppTheme>(12.0, colors.card_bg),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(Message::System(SystemMessage::OpenLearnMoreFingerprint))
    .padding(Padding::from([10, 20]))
    .width(Length::Fill);

    let close_button = buttons::secondary(
        text(fl!("menu-fingerprint-close"))
            .size(14)
            .color(colors.accent),
    )
    .on_press(Message::System(SystemMessage::CloseFingerprintModal))
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
        Message::System(SystemMessage::CloseFingerprintModal),
    ))
}

/// Open the fingerprint help page in the default browser.
pub fn open_learn_more() {
    clipboard::launch_url(LEARN_MORE_URL);
}
