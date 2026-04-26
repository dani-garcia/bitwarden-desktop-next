use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    services::clipboard,
    views::vault::VaultEvent,
};

impl App {
    pub(crate) fn handle_vault_event(&mut self, event: VaultEvent) -> Task<Message> {
        match event {
            VaultEvent::AccountSwitcher(e) => self.handle_account_switcher_event(e),
            VaultEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
            VaultEvent::ItemSaved { uid } => {
                self.push_toast(Toast::success(fl!("vault-toast-item-saved"), None));
                self.load_vault_list_task(uid)
            }
            VaultEvent::ItemDeleted { uid } => {
                self.push_toast(Toast::success(fl!("vault-toast-item-deleted"), None));
                self.load_vault_list_task(uid)
            }
            VaultEvent::ClipboardCopyRequested {
                value,
                sensitivity,
                toast_label,
            } => {
                self.clipboard.copy(value, sensitivity);
                self.push_toast(Toast::success(toast_label, None));
                let minimize = self
                    .active_user
                    .as_ref()
                    .map(|uid| self.settings.preferences_for(uid).minimize_on_copy)
                    .unwrap_or(false);
                if minimize {
                    iced::window::minimize(self.main_window_id(), true)
                } else {
                    Task::none()
                }
            }
            VaultEvent::LaunchUrlRequested { uri } => {
                clipboard::launch_url(&uri);
                Task::none()
            }
        }
    }
}
