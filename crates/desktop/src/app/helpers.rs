use iced::Task;

use crate::{
    domain::{Screen, UserId},
    services::sdk::AccountEntry,
};

use super::{App, Message, WindowKind};

impl App {
    pub(crate) fn main_window_id(&self) -> iced::window::Id {
        self.main_window
    }

    pub(crate) fn about_window_id(&self) -> Option<iced::window::Id> {
        self.windows
            .iter()
            .find(|(_, w)| w.kind == WindowKind::About)
            .map(|(id, _)| *id)
    }

    pub(crate) fn main_window_maximized(&self) -> bool {
        self.windows
            .get(&self.main_window)
            .is_some_and(|w| w.maximized)
    }

    pub(crate) fn push_toast(&mut self, toast: crate::components::toast::Toast) {
        self.toasts.push(toast);
    }

    pub(crate) fn active_account_entry(&self) -> Option<&AccountEntry> {
        self.active_user
            .as_ref()
            .and_then(|uid| self.cache.accounts.iter().find(|a| &a.user_id == uid))
    }

    /// Repopulate the accounts snapshot + resync the native menu's enabled
    /// state. Called from handlers that mutate SDK-side user state (login,
    /// logout, lock, unlock, user switch, manager load).
    pub(crate) fn refresh_accounts_cache(&mut self) {
        self.cache.accounts = self.client_manager.accounts();

        if let Some(ref handle) = self.native_menu {
            handle.sync_enabled(&self.menu_state());
        }
    }

    /// Assign a new screen plus standard transition bookkeeping: drop the
    /// previous overlay, resync the accounts cache + native menu state.
    /// Callers do view-specific pre-work (e.g. `show_unlock_for`) first.
    pub(crate) fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.open_overlay = None;
        self.refresh_accounts_cache();
    }

    /// Lock a user's keystore. If `uid` is the active user, also transitions
    /// to the login screen with their unlock prompt; otherwise the call is
    /// silent and the lock is reflected next time the user is selected.
    pub(crate) fn lock_user(&mut self, uid: &UserId) -> Task<Message> {
        self.client_manager.lock(uid);
        if self.active_user.as_ref() == Some(uid) {
            self.show_login_after_lock()
        } else {
            Task::none()
        }
    }

    /// Standard post-lock transition: drop sticky Magnify state, route the
    /// login view to the active user's unlock prompt, switch to
    /// [`Screen::Login`], and return the auto-focus task.
    fn show_login_after_lock(&mut self) -> Task<Message> {
        self.magnify_reset_sticky();
        self.views
            .login
            .show_unlock_for(self.active_user.as_ref(), &self.client_manager);
        self.set_screen(Screen::Login);
        self.views.login.auto_focus_task().map(Message::login)
    }

    /// Lift `VaultView::load_list_task` into a top-level `Task<Message>`,
    /// hiding the per-call-site `.map(Message::vault)`.
    pub(crate) fn load_vault_list_task(&self, uid: UserId) -> Task<Message> {
        crate::views::vault::VaultView::load_list_task(uid, &self.client_manager)
            .map(Message::vault)
    }

    pub(crate) fn load_send_list_task(&self, uid: UserId) -> Task<Message> {
        crate::views::send::SendView::load_list_task(uid, &self.client_manager).map(Message::send)
    }

    pub(crate) fn menu_state(&self) -> crate::services::menu::MenuState {
        let has_accounts = self.client_manager.has_users();
        let is_locked = self
            .active_user
            .as_ref()
            .map(|uid| !self.client_manager.is_unlocked(uid))
            .unwrap_or(true);
        let has_lockable = self.client_manager.has_unlocked_users();

        crate::services::menu::MenuState {
            is_locked,
            has_accounts,
            has_lockable_accounts: has_lockable,
        }
    }
}

// ── Platform-specific window settings ─────────────────────────────────────
// `iced::daemon` doesn't take a `.window(Settings)` — boot creates the window
// via `window::open` and passes this through.

pub(crate) fn main_window_platform_specific() -> iced::window::settings::PlatformSpecific {
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
