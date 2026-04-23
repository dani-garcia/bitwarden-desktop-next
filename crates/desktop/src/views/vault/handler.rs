use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    domain::Screen,
    fl,
    services::{clipboard, menu::MenuAction},
    views::vault::VaultEvent,
};

impl App {
    pub(crate) fn handle_vault_event(&mut self, event: VaultEvent) -> Task<Message> {
        match event {
            VaultEvent::UserSelected { uid } => self.handle_user_switch(uid),
            VaultEvent::AddAccountRequested => {
                self.screen = Screen::Login;
                self.login_view.reset_to_email_entry();
                Task::none()
            }
            VaultEvent::LockAllRequested => self.handle_menu_action(MenuAction::LockAllVaults),
            VaultEvent::SettingsRequested => self.handle_menu_action(MenuAction::Settings),
            VaultEvent::LockActiveRequested => self.handle_lock_active(),
            VaultEvent::SignOutRequested => self.handle_sign_out(),
            VaultEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
            VaultEvent::ItemSaved { uid } => {
                self.push_toast(Toast::success(fl!("vault-toast-item-saved"), None));
                // Refresh the list so renamed items / ownership changes
                // show up in the left pane without a manual reload.
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
                // `minimize_on_copy` is a per-user preference — trigger it
                // only when the active user has opted in.
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
