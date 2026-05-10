//! Settings → Allow screenshots confirm-still-visible modal handler.
//!
//! Two messages, two effects:
//!
//! - `Confirm`: user clicked OK in time. Close the modal; protection
//!   stays as-is (= ON). Closing also drops the abort handle, which
//!   cancels the still-pending 5 s sleep so its `Timeout` never lands.
//! - `Timeout`: 5 s elapsed without confirmation. Revert: turn
//!   protection back off, un-apply at the OS level, persist, close the
//!   modal. The `is_open()` guard catches the rare case where the abort
//!   raced with delivery (handle dropped *just* as the sleep completed
//!   and the message was already in iced's queue).

use iced::Task;

use crate::{
    app::{App, Message},
    views::screenshot_confirm::ScreenshotConfirmMessage,
};

impl App {
    pub(crate) fn handle_screenshot_confirm_message(
        &mut self,
        msg: ScreenshotConfirmMessage,
    ) -> Task<Message> {
        match msg {
            ScreenshotConfirmMessage::Confirm => {
                self.screenshot_confirm.close();
                Task::none()
            }
            ScreenshotConfirmMessage::Timeout => {
                if !self.screenshot_confirm.is_open() {
                    // Abort raced with delivery — drop the stale message.
                    return Task::none();
                }
                // User couldn't see / didn't react. Revert protection,
                // mirror the change into the settings view's snapshot
                // (so the checkbox flips back visually if the settings
                // modal happens to be open), and un-apply at the OS level.
                self.screenshot_confirm.close();
                self.settings.allow_screenshots = true;
                self.settings.save();
                self.views.settings.revert_allow_screenshots();
                crate::services::screenshot_protection::apply(self.main_window, false)
                    .discard()
            }
        }
    }
}
