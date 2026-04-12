use std::sync::Arc;

use iced::Task;

use crate::{
    components::account_switcher::AccountEntry,
    state::{Screen, UnlockMethod, UserId, UserSession},
    views::{login::AuthPage, vault::VaultMessage},
};

use super::{App, Message, WindowKind};

impl App {
    pub(super) fn main_window_id(&self) -> Option<iced::window::Id> {
        self.windows
            .iter()
            .find(|(_, w)| w.kind == WindowKind::Main)
            .map(|(id, _)| *id)
    }

    pub(super) fn about_window_id(&self) -> Option<iced::window::Id> {
        self.windows
            .iter()
            .find(|(_, w)| w.kind == WindowKind::About)
            .map(|(id, _)| *id)
    }

    pub(super) fn main_window_maximized(&self) -> bool {
        self.main_window_id()
            .and_then(|id| self.windows.get(&id))
            .is_some_and(|w| w.maximized)
    }

    pub(super) fn post_update(&mut self) {
        self.refresh_cache();
    }

    pub(super) fn push_toast(&mut self, toast: crate::components::toast::Toast) {
        self.toasts.push(toast);
    }

    pub(super) fn active_session(&self) -> Option<&UserSession> {
        self.state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid))
    }

    pub(super) fn refresh_cache(&mut self) {
        let active_session = self.active_session().cloned();

        self.cache.email = active_session
            .as_ref()
            .map(|s| s.email.clone())
            .unwrap_or_else(|| "No account".into());

        self.cache.server = active_session
            .as_ref()
            .map(|s| s.server_url.clone())
            .unwrap_or_default();

        self.cache.accounts = self
            .state
            .users
            .iter()
            .map(|(uid, session)| AccountEntry {
                user_id: uid.clone(),
                email: session.email.clone(),
                server_url: session.server_url.clone(),
                locked: session.locked,
            })
            .collect();

        // Compute unlock alternatives for the current auth page
        self.cache.unlock_alternatives = if let AuthPage::Unlock(method) = self.login_view.auth_page
        {
            active_session
                .as_ref()
                .map(|s| s.unlock_methods.alternatives(method))
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        if let Some(ref handle) = self.native_menu {
            crate::menu::sync_native_enabled(handle, &self.menu_state());
        }
    }

    /// Switch the active user. Clears the previous user's vault state and,
    /// if the new user is unlocked, returns the task that repopulates the
    /// vault list.
    pub(super) fn handle_user_switch(&mut self, uid: UserId) -> Task<Message> {
        self.state.active_user = Some(uid.clone());
        let session = self.state.users.get(&uid);
        let locked = session.map(|s| s.locked).unwrap_or(true);
        self.vault_view.reset();
        if locked {
            self.state.screen = Screen::Login;
            let preferred = session
                .map(|s| s.unlock_methods.preferred())
                .unwrap_or(UnlockMethod::MasterPassword);
            self.login_view.auth_page = AuthPage::Unlock(preferred);
            self.login_view.password_input.clear();
            self.login_view.pin_input.clear();
            self.login_view.show_password = false;
            Task::none()
        } else {
            self.state.screen = Screen::Vault;
            self.load_vault_list_task(uid)
        }
    }

    /// Build the task that decrypts the user's vault list and lands as
    /// `VaultMessage::ListLoaded`. Returns `Task<Message>` (already lifted
    /// via `.map(Message::Vault)`) so all callers can plumb the result
    /// without caring about the inner message type.
    pub(super) fn load_vault_list_task(&self, uid: UserId) -> Task<Message> {
        let mgr = self.client_manager.clone();
        let uid_for_msg = uid.clone();
        Task::perform(
            async move {
                mgr.list_ciphers(&uid)
                    .await
                    .map(|items| items.into_iter().map(Arc::new).collect::<Vec<_>>())
            },
            move |result| VaultMessage::ListLoaded(uid_for_msg.clone(), result),
        )
        .map(Message::Vault)
    }

    pub(super) fn menu_state(&self) -> crate::menu::MenuState {
        let has_accounts = !self.state.users.is_empty();
        let is_locked = self
            .state
            .active_user
            .as_ref()
            .and_then(|uid| self.state.users.get(uid))
            .map(|s| s.locked)
            .unwrap_or(true);
        let has_lockable = self.state.users.values().any(|s| !s.locked);

        crate::menu::MenuState {
            is_locked,
            has_accounts,
            has_lockable_accounts: has_lockable,
            has_authenticated_accounts: has_accounts,
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
