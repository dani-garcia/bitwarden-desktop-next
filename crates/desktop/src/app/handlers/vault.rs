use iced::Task;

use crate::{
    clipboard,
    components::toast::Toast,
    state::Screen,
    views::vault::VaultEvent,
};

use super::super::{App, Message};

impl App {
    pub(in crate::app) fn handle_vault_event(&mut self, event: VaultEvent) -> Task<Message> {
        match event {
            VaultEvent::UserSelected { uid } => self.handle_user_switch(uid),
            VaultEvent::AddAccountRequested => {
                self.screen = Screen::Login;
                self.login_view.reset_to_email_entry();
                Task::none()
            }
            VaultEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
            VaultEvent::ClipboardCopyRequested {
                value,
                sensitivity,
                toast_label,
            } => {
                self.clipboard.copy(value, sensitivity);
                self.push_toast(Toast::success(toast_label, None));
                Task::none()
            }
            VaultEvent::LaunchUrlRequested { uri } => {
                clipboard::launch_url(&uri);
                Task::none()
            }
        }
    }
}
