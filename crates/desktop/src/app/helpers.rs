use iced::Task;

use crate::{
    components::account_switcher::AccountEntry,
    state::{Screen, UserId},
    views::login::AuthPage,
};

use super::{App, Message, WindowKind};

impl App {
    pub(super) fn main_window_id(&self) -> iced::window::Id {
        self.main_window
    }

    pub(super) fn about_window_id(&self) -> Option<iced::window::Id> {
        self.windows
            .iter()
            .find(|(_, w)| w.kind == WindowKind::About)
            .map(|(id, _)| *id)
    }

    pub(super) fn main_window_maximized(&self) -> bool {
        self.windows
            .get(&self.main_window)
            .is_some_and(|w| w.maximized)
    }

    pub(super) fn post_update(&mut self) {
        self.refresh_cache();
    }

    pub(super) fn push_toast(&mut self, toast: crate::components::toast::Toast) {
        self.toasts.push(toast);
    }

    /// Close every overlay owned by a sub-view or the title bar. Distinct
    /// from the router's cross-view dismissal arms, which only close the
    /// *other* view's overlays — this one is for "opening something on top
    /// of everything" cases like the settings modal.
    pub(super) fn dismiss_all_overlays(&mut self) {
        self.title_bar.dismiss_menu();
        self.login_view.dismiss_dropdowns();
        self.vault_view.dismiss_dropdowns();
    }

    pub(super) fn active_account_entry(&self) -> Option<&AccountEntry> {
        self.active_user
            .as_ref()
            .and_then(|uid| self.cache.accounts.iter().find(|a| &a.user_id == uid))
    }

    pub(super) fn refresh_cache(&mut self) {
        self.cache.accounts = self
            .client_manager
            .user_ids()
            .map(|uid| AccountEntry {
                user_id: *uid,
                email: self.client_manager.email(uid).unwrap_or("").to_string(),
                display_name: self
                    .client_manager
                    .display_name(uid)
                    .unwrap_or("")
                    .to_string(),
                server_url: self
                    .client_manager
                    .server_url(uid)
                    .unwrap_or("")
                    .to_string(),
                locked: !self.client_manager.is_unlocked(uid),
            })
            .collect();

        // Compute unlock alternatives for the current auth page
        self.cache.unlock_alternatives =
            if let AuthPage::Unlock { method, .. } = self.login_view.auth_page {
                self.active_user
                    .as_ref()
                    .and_then(|uid| self.client_manager.unlock_methods(uid))
                    .map(|m| m.alternatives(method))
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

        if let Some(ref handle) = self.native_menu {
            handle.sync_enabled(&self.menu_state());
        }
    }

    /// Switch the active user. Clears the previous user's vault state and,
    /// if the new user is unlocked, returns the task that repopulates the
    /// vault list.
    pub(super) fn handle_user_switch(&mut self, uid: UserId) -> Task<Message> {
        self.active_user = Some(uid);
        self.vault_view.reset(&uid);
        // Re-apply the new user's clipboard clear delay so the app-global
        // clipboard manager matches their preference.
        let delay = self.settings.preferences_for(&uid).clear_clipboard;
        self.clipboard.set_timeout(delay.as_duration());

        if !self.client_manager.is_unlocked(&uid) {
            self.screen = Screen::Login;
            self.login_view
                .show_unlock_for(Some(&uid), &self.client_manager);
            Task::none()
        } else {
            self.screen = Screen::Vault;
            self.load_vault_list_task(uid)
        }
    }

    /// Lift `VaultView::load_list_task` into a top-level `Task<Message>`.
    /// Thin wrapper so handlers don't have to repeat the `.map(Message::Vault)`
    /// lift at each call site.
    pub(super) fn load_vault_list_task(&self, uid: UserId) -> Task<Message> {
        crate::views::vault::VaultView::load_list_task(uid, &self.client_manager)
            .map(Message::Vault)
    }

    pub(super) fn menu_state(&self) -> crate::menu::MenuState {
        let has_accounts = self.client_manager.has_users();
        let is_locked = self
            .active_user
            .as_ref()
            .map(|uid| !self.client_manager.is_unlocked(uid))
            .unwrap_or(true);
        let has_lockable = self.client_manager.has_unlocked_users();

        crate::menu::MenuState {
            is_locked,
            has_accounts,
            has_lockable_accounts: has_lockable,
        }
    }
}

// ── Platform-specific window settings ─────────────────────────────────────
//
// Used by `App::new` to configure the main window. `iced::daemon` doesn't take
// a `.window(Settings)` — boot creates the window via `window::open`.

pub(super) fn main_window_platform_specific() -> iced::window::settings::PlatformSpecific {
    #[cfg(target_os = "windows")]
    {
        use iced::window::settings::PlatformSpecific;
        PlatformSpecific {
            undecorated_shadow: true,
            corner_preference: iced::window::settings::platform::CornerPreference::Round,
            ..Default::default()
        }
    }

    #[cfg(target_os = "macos")]
    {
        use iced::window::settings::PlatformSpecific;
        PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        iced::window::settings::PlatformSpecific::default()
    }
}
