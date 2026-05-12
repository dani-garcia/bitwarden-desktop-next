//! Settings → Allow screenshots confirm-still-visible event handler.
//!
//! The view handles Confirm (close in place) and the stale-Timeout guard
//! inline. The Revert path bubbles out because it mutates the persisted
//! [`Settings`], re-syncs the settings view's snapshot, and re-applies
//! protection at the OS level — all state outside [`UpdateCtx`].
//!
//! [`Settings`]: crate::services::settings::Settings

use iced::Task;

use crate::{
    app::{App, Message},
    views::screenshot_confirm::ScreenshotConfirmEvent,
};

impl App {
    pub(crate) fn handle_screenshot_confirm_event(
        &mut self,
        event: ScreenshotConfirmEvent,
    ) -> Task<Message> {
        match event {
            ScreenshotConfirmEvent::Revert => {
                // User couldn't see / didn't react. Revert protection,
                // mirror the change into the settings view's snapshot
                // (so the checkbox flips back visually if the settings
                // modal happens to be open), and un-apply at the OS level.
                self.settings.allow_screenshots = true;
                self.settings.save();
                self.views.settings.revert_allow_screenshots();
                crate::services::screenshot_protection::apply(self.main_window, false).discard()
            }
        }
    }
}
