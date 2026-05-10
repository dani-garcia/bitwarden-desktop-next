//! Post-toggle confirmation dialog for screen-capture protection.
//!
//! Shown the moment the user enables `Block screenshots` in Settings →
//! Advanced. The whole purpose: catch the case where the user is on a
//! remote-desktop session (or any other capture-relaying setup) and
//! enabling protection has just hidden the window from them. If they
//! can read the dialog, they click OK and the setting sticks. If they
//! can't (because the window is now invisible to their capture), the
//! 5-second timeout reverts protection automatically and they regain
//! control.
//!
//! The settings handler builds an [`iced::Task::abortable`] sleep,
//! hands the [`iced::task::Handle`] to [`ScreenshotConfirmModal::open`],
//! and dispatches the resulting Task. Closing the modal (or replacing
//! the handle on a fresh open) drops the prior `Handle`, whose
//! `abort_on_drop` cancels the in-flight sleep — so the only `Timeout`
//! message that ever reaches the handler is one whose modal is still
//! open.

use std::time::Duration;

use iced::{
    Alignment, Border, Color, Element, Fill, Length, Padding, Shadow, Vector,
    widget::{Space, column, container, row, text},
};

use crate::{
    components::{FadeInOut, buttons, icons, modal},
    fl,
    theme::{AppColors, AppTheme},
};

/// How long the dialog gives the user to confirm before auto-reverting.
/// Bitwarden Electron uses 10 s; this client trims to 5 s — short enough
/// that a user who can't see the dialog gets their window back quickly,
/// long enough for a user who can see it to react.
pub const REVERT_AFTER: Duration = Duration::from_secs(5);

/// Decorative ring around the icon. Matches the fingerprint-phrase modal.
const RING_DIAMETER: f32 = 48.0;

#[derive(Debug, Clone, Copy)]
pub enum ScreenshotConfirmMessage {
    /// User clicked OK — keep protection on.
    Confirm,
    /// `REVERT_AFTER` elapsed without confirmation. The handle stored on
    /// the modal aborts the underlying Task on `close()` / re-`open()`,
    /// so this variant only ever lands while the modal is genuinely
    /// open and unconfirmed.
    Timeout,
}

#[derive(Default)]
pub struct ScreenshotConfirmModal {
    fade: FadeInOut,
    /// Auto-aborts the in-flight 5 s sleep when the handle is dropped —
    /// either because the user closed the modal (`close()`) or because
    /// a fresh `open()` overwrote the slot. `None` outside an open
    /// session.
    pending_revert: Option<iced::task::Handle>,
}

impl ScreenshotConfirmModal {
    /// Open the dialog and store the abort handle for the matching
    /// in-flight 5 s timeout. Replacing the handle drops the previous
    /// one, which `abort_on_drop` then cancels — so a stale `Timeout`
    /// from an earlier session can't survive into a new one.
    pub fn open(&mut self, revert_handle: iced::task::Handle) {
        self.pending_revert = Some(revert_handle.abort_on_drop());
        self.fade.open();
    }

    pub fn close(&mut self) {
        // Drop the handle → abort_on_drop cancels the pending sleep.
        self.pending_revert = None;
        self.fade.close();
    }

    pub fn is_open(&self) -> bool {
        self.fade.is_open()
    }
}

/// Returns `None` while fully closed; App's view code drops the slot
/// rather than rendering an invisible layer.
pub fn modal_view<'a>(
    state: &'a ScreenshotConfirmModal,
    colors: &'a AppColors,
) -> Option<Element<'a, ScreenshotConfirmMessage, AppTheme>> {
    let progress = state.fade.progress_if_visible()?;

    let icon_ring = container(
        icons::INFO_CIRCLE_FILL
            .render::<ScreenshotConfirmMessage, AppTheme>(28.0, colors.accent),
    )
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

    let title = text(fl!("settings-confirm-window-visible-title"))
        .size(16)
        .color(colors.text_primary)
        .font(crate::APP_FONT_BOLD);

    let body_text = text(fl!("settings-confirm-window-visible-body"))
        .size(14)
        .color(colors.text_secondary);

    let ok_button = buttons::primary(text(fl!("settings-confirm-window-visible-ok")).size(14))
        .on_press(ScreenshotConfirmMessage::Confirm)
        .padding(Padding::from([10, 20]))
        .width(Length::Fill);

    let body = column![
        container(icon_ring).width(Fill).align_x(Alignment::Center),
        Space::new().height(Length::Fixed(8.0)),
        column![title, body_text]
            .spacing(8)
            .align_x(Alignment::Center)
            .width(Fill),
        Space::new().height(Length::Fixed(8.0)),
        row![ok_button].spacing(8).width(Fill),
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
        420.0,
        None,
        |c| c.card_bg,
        progress,
        body,
        ScreenshotConfirmMessage::Confirm,
    ))
}
