use iced::Task;

use crate::{
    components::account_switcher::AccountSwitcherEvent,
    domain::{Screen, UserId},
    services::{menu::MenuAction, sdk::AccountEntry},
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
    /// state. Called from the handful of handlers that mutate SDK-side user
    /// state (login, logout, lock, unlock, user switch, manager load) — the
    /// invalidation set is identical for both, hence the shared entry point.
    pub(crate) fn refresh_accounts_cache(&mut self) {
        self.cache.accounts = self.client_manager.accounts();

        if let Some(ref handle) = self.native_menu {
            handle.sync_enabled(&self.menu_state());
        }
    }

    /// Assign a new screen plus the bookkeeping that always goes with a
    /// screen transition: drop whatever overlay was open on the previous
    /// screen, resync the accounts cache + native menu state. Callers do any
    /// view-specific pre-work (e.g. `show_unlock_for`, `reset_to_email_entry`)
    /// before calling this.
    pub(crate) fn set_screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.open_overlay = None;
        self.refresh_accounts_cache();
    }

    /// Route any account-switcher action. Called from both login and vault
    /// event handlers so the semantics live in exactly one place.
    pub(crate) fn handle_account_switcher_event(
        &mut self,
        event: AccountSwitcherEvent,
    ) -> Task<Message> {
        match event {
            AccountSwitcherEvent::SwitchUser { uid } => self.handle_user_switch(uid),
            AccountSwitcherEvent::AddAccount => {
                self.views.login.reset_to_email_entry();
                self.set_screen(Screen::Login);
                Task::none()
            }
            AccountSwitcherEvent::LockAll => self.handle_menu_action(MenuAction::LockAllVaults),
            AccountSwitcherEvent::Settings => self.handle_menu_action(MenuAction::Settings),
            AccountSwitcherEvent::LockActive => self.handle_lock_active(),
            AccountSwitcherEvent::LogOut => self.handle_log_out(),
        }
    }

    /// Switch the active user. Clears the previous user's vault state and,
    /// if the new user is unlocked, returns the task that repopulates the
    /// vault list.
    pub(crate) fn handle_user_switch(&mut self, uid: UserId) -> Task<Message> {
        self.active_user = Some(uid);
        self.views.vault.reset(&uid);
        // Re-apply the new user's clipboard clear delay so the app-global
        // clipboard manager matches their preference.
        let delay = self.settings.preferences_for(&uid).clear_clipboard;
        self.clipboard.set_timeout(delay.as_duration());

        if !self.client_manager.is_unlocked(&uid) {
            self.views
                .login
                .show_unlock_for(Some(&uid), &self.client_manager);
            self.set_screen(Screen::Login);
            Task::none()
        } else {
            self.set_screen(Screen::Vault);
            self.load_vault_list_task(uid)
        }
    }

    /// Lift `VaultView::load_list_task` into a top-level `Task<Message>`.
    /// Thin wrapper so handlers don't have to repeat the `.map(Message::vault)`
    /// lift at each call site.
    pub(crate) fn load_vault_list_task(&self, uid: UserId) -> Task<Message> {
        crate::views::vault::VaultView::load_list_task(uid, &self.client_manager)
            .map(Message::vault)
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
//
// Used by `App::new` to configure the main window. `iced::daemon` doesn't take
// a `.window(Settings)` — boot creates the window via `window::open`.

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
