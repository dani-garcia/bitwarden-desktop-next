mod layout;
mod login_email;
mod login_password;
mod server_selector;
mod unlock;

use std::sync::Arc;

use iced::{Element, Task};

use crate::{
    components::{
        account_switcher::{AccountEntry, AccountSwitcherMessage},
        toast::Toast,
    },
    sdk::ClientManager,
    state::{UnlockMethod, UserId},
    theme::{AppColors, AppTheme},
};

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AuthPage {
    Unlock {
        method: UnlockMethod,
        password_input: String,
        pin_input: String,
    },
    LoginEmail {
        email_input: String,
        remember_email: bool,
        server_selector_open: bool,
        selected_server: ServerOption,
    },
    LoginPassword {
        email: String,
        password_input: String,
        selected_server: ServerOption,
    },
}

impl AuthPage {
    pub fn new_unlock(method: UnlockMethod) -> Self {
        Self::Unlock {
            method,
            password_input: String::new(),
            pin_input: String::new(),
        }
    }

    pub fn new_login_email() -> Self {
        Self::LoginEmail {
            email_input: String::new(),
            remember_email: false,
            server_selector_open: false,
            selected_server: ServerOption::Bitwarden,
        }
    }

    pub fn new_login_password(email: String, selected_server: ServerOption) -> Self {
        Self::LoginPassword {
            email,
            password_input: String::new(),
            selected_server,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerOption {
    Bitwarden,
    BitwardenEu,
    SelfHosted(String),
}

impl ServerOption {
    pub fn display_name(&self) -> String {
        match self {
            ServerOption::Bitwarden => "bitwarden.com".to_string(),
            ServerOption::BitwardenEu => "bitwarden.eu".to_string(),
            ServerOption::SelfHosted(url) if url.is_empty() => {
                crate::fl!("login-server-self-hosted")
            }
            ServerOption::SelfHosted(url) => url.clone(),
        }
    }
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LoginMessage {
    // Unlock — master password
    PasswordChanged(String),
    Unlock,
    /// Fires when the async `ClientManager::unlock` task completes.
    UnlockCompleted(UserId, Result<(), String>),
    // Unlock — PIN
    PinChanged(String),
    UnlockWithPin,
    // Unlock — biometrics
    UnlockWithBiometrics,
    // Switch between unlock methods
    SwitchUnlockMethod(UnlockMethod),
    LogOut,

    // Login — email entry
    EmailChanged(String),
    ToggleRememberEmail(bool),
    ContinueWithEmail,
    UseSingleSignOn,

    // Login — password entry
    LoginPasswordChanged(String),
    LoginWithPassword,
    /// Fires when the async `ClientManager::login` task completes.
    /// TODO: wire up once SDK login support lands.
    #[expect(dead_code)]
    LoginCompleted(UserId, Result<(), String>),
    BackToEmail,
    GetPasswordHint,

    // Server selector
    ToggleServerSelector,
    SelectServer(ServerOption),

    // Account switcher
    AccountSwitcher(AccountSwitcherMessage),
}

// ── Events ─────────────────────────────────────────────────────────────────
//
// Events are declarative domain facts the view bubbles up for cross-cutting
// effects App needs to coordinate — screen switches, user swaps, toast
// pushes. Compare with the older `Action` pattern which was imperative
// ("App please do X"); the event form lets the view own its async lifecycle
// and only surface the *completed* state transitions.

#[derive(Debug, Clone)]
pub enum LoginEvent {
    /// Unlock attempt completed successfully — App should flip to the vault
    /// screen and kick off the vault list load for this user.
    Unlocked { uid: UserId },
    /// Fresh login via email + password completed successfully. Same routing
    /// as `Unlocked` but distinguishes the flow for future analytics / error
    /// messaging differences. Constructed only by `LoginMessage::LoginCompleted`
    /// which is itself a stub until SDK login support lands.
    LoggedIn { uid: UserId },
    /// User clicked Sign Out — App should drop the active session.
    SignOutRequested,
    /// User picked a different account from the switcher.
    UserSelected { uid: UserId },
    /// LoginView wants to show a cross-cutting toast notification.
    ToastRequested(Toast),
}

// ── View State ─────────────────────────────────────────────────────────────

pub struct LoginView {
    pub auth_page: AuthPage,
    pub account_switcher_open: bool,
    /// True between `LoginMessage::Unlock` firing and `UnlockCompleted` arriving.
    /// Drives the in-progress spinner on the unlock screen and gates re-entry.
    pub unlock_in_progress: bool,
}

impl LoginView {
    pub fn new() -> Self {
        Self {
            auth_page: AuthPage::new_unlock(UnlockMethod::MasterPassword),
            account_switcher_open: false,
            unlock_in_progress: false,
        }
    }

    /// Compositional MVU update. Returns a task (for async work the view
    /// owns) and an optional event (cross-cutting fact for App to route).
    ///
    /// `client_manager` and `active_user` are injected at call-time so the
    /// view can construct `Task::perform` calls without owning shared state.
    /// This matches Halloy's pattern — see
    /// [investigation/halloy/src/buffer.rs:247](../../../../investigation/halloy/src/buffer.rs).
    pub fn update(
        &mut self,
        msg: LoginMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> (Task<LoginMessage>, Option<LoginEvent>) {
        match msg {
            // ── Unlock: master password ────────────────────────────────────
            LoginMessage::PasswordChanged(pw) => {
                if let AuthPage::Unlock { password_input, .. } = &mut self.auth_page {
                    *password_input = pw;
                }
                (Task::none(), None)
            }
            LoginMessage::Unlock => {
                // Re-entry guard: ignore Enter-spam while a task is already
                // in flight (the text_input's on_submit fires per keystroke).
                if self.unlock_in_progress {
                    return (Task::none(), None);
                }
                let password = if let AuthPage::Unlock { password_input, .. } = &mut self.auth_page
                {
                    std::mem::take(password_input)
                } else {
                    return (Task::none(), None);
                };
                let Some(uid) = active_user.cloned() else {
                    return (Task::none(), None);
                };
                self.unlock_in_progress = true;
                let mgr = client_manager.clone();
                let task = Task::perform(
                    async move { mgr.unlock(&uid, password).await },
                    move |res| LoginMessage::UnlockCompleted(uid, res),
                );
                (task, None)
            }
            LoginMessage::UnlockCompleted(msg_uid, result) => {
                self.unlock_in_progress = false;
                // Stale-check: the user may have switched accounts while the
                // unlock was in flight. Drop stale results so they don't
                // clobber a new active session.
                if active_user != Some(&msg_uid) {
                    tracing::debug!(
                        uid = %msg_uid,
                        "unlock result dropped: active user changed while in flight"
                    );
                    return (Task::none(), None);
                }
                match result {
                    Ok(()) => (Task::none(), Some(LoginEvent::Unlocked { uid: msg_uid })),
                    Err(err) => {
                        // Log the raw SDK error for debugging, but show a
                        // sanitized message to the user — SDK `Display`
                        // impls can surface internal crypto state / module
                        // paths that shouldn't land in a toast notification.
                        tracing::warn!(
                            uid = %msg_uid,
                            %err,
                            "SDK initialize_user_crypto failed"
                        );
                        (
                            Task::none(),
                            Some(LoginEvent::ToastRequested(Toast::error(
                                crate::fl!("login-toast-unlock-failed-body"),
                                Some(&crate::fl!("login-toast-unlock-failed-title")),
                            ))),
                        )
                    }
                }
            }

            // ── Unlock: PIN ────────────────────────────────────────────────
            LoginMessage::PinChanged(pin) => {
                if let AuthPage::Unlock { pin_input, .. } = &mut self.auth_page {
                    *pin_input = pin;
                }
                (Task::none(), None)
            }
            LoginMessage::UnlockWithPin => {
                if let AuthPage::Unlock { pin_input, .. } = &mut self.auth_page {
                    pin_input.clear();
                }
                (
                    Task::none(),
                    Some(LoginEvent::ToastRequested(Toast::warning(
                        crate::fl!("login-toast-pin-unsupported"),
                        None,
                    ))),
                )
            }

            // ── Unlock: biometrics ─────────────────────────────────────────
            LoginMessage::UnlockWithBiometrics => (
                Task::none(),
                Some(LoginEvent::ToastRequested(Toast::warning(
                    crate::fl!("login-toast-biometrics-unsupported"),
                    None,
                ))),
            ),

            // ── Switch unlock method ───────────────────────────────────────
            LoginMessage::SwitchUnlockMethod(method) => {
                self.auth_page = AuthPage::new_unlock(method);
                self.unlock_in_progress = false;
                (Task::none(), None)
            }

            LoginMessage::LogOut => (Task::none(), Some(LoginEvent::SignOutRequested)),

            // ── Login: email entry ─────────────────────────────────────────
            LoginMessage::EmailChanged(email) => {
                if let AuthPage::LoginEmail { email_input, .. } = &mut self.auth_page {
                    *email_input = email;
                }
                (Task::none(), None)
            }
            LoginMessage::ToggleRememberEmail(checked) => {
                if let AuthPage::LoginEmail { remember_email, .. } = &mut self.auth_page {
                    *remember_email = checked;
                }
                (Task::none(), None)
            }
            LoginMessage::ContinueWithEmail => {
                if let AuthPage::LoginEmail {
                    email_input,
                    selected_server,
                    ..
                } = &mut self.auth_page
                {
                    let email = email_input.clone();
                    let server = selected_server.clone();
                    self.auth_page = AuthPage::new_login_password(email, server);
                }
                (Task::none(), None)
            }
            LoginMessage::UseSingleSignOn => {
                // TODO: SSO login flow
                (Task::none(), None)
            }

            // ── Login: password entry ──────────────────────────────────────
            LoginMessage::LoginPasswordChanged(pw) => {
                if let AuthPage::LoginPassword { password_input, .. } = &mut self.auth_page {
                    *password_input = pw;
                }
                (Task::none(), None)
            }
            LoginMessage::LoginWithPassword => {
                // TODO: call `client_manager.login(email, password).await`
                // once SDK login support lands. For now this is a stub — we
                // clear the input and emit nothing so the button press is
                // visibly consumed.
                if let AuthPage::LoginPassword { password_input, .. } = &mut self.auth_page {
                    let _password = std::mem::take(password_input);
                }
                (Task::none(), None)
            }
            LoginMessage::LoginCompleted(msg_uid, result) => {
                if active_user != Some(&msg_uid) {
                    tracing::debug!(
                        uid = %msg_uid,
                        "login result dropped: active user changed while in flight"
                    );
                    return (Task::none(), None);
                }
                match result {
                    Ok(()) => (Task::none(), Some(LoginEvent::LoggedIn { uid: msg_uid })),
                    Err(err) => {
                        tracing::warn!(uid = %msg_uid, %err, "SDK login failed");
                        (
                            Task::none(),
                            Some(LoginEvent::ToastRequested(Toast::error(
                                err,
                                Some(&crate::fl!("login-toast-login-failed-title")),
                            ))),
                        )
                    }
                }
            }
            LoginMessage::BackToEmail => {
                // Preserve the email + selected server when going back so
                // the user doesn't have to retype what they just entered.
                let (email, server) = match &self.auth_page {
                    AuthPage::LoginPassword {
                        email,
                        selected_server,
                        ..
                    } => (email.clone(), selected_server.clone()),
                    _ => (String::new(), ServerOption::Bitwarden),
                };
                self.auth_page = AuthPage::LoginEmail {
                    email_input: email,
                    remember_email: false,
                    server_selector_open: false,
                    selected_server: server,
                };
                (Task::none(), None)
            }
            LoginMessage::GetPasswordHint => {
                // TODO: password hint request flow
                (Task::none(), None)
            }

            // ── Server selector ────────────────────────────────────────────
            LoginMessage::ToggleServerSelector => {
                if let AuthPage::LoginEmail {
                    server_selector_open,
                    ..
                } = &mut self.auth_page
                {
                    *server_selector_open = !*server_selector_open;
                }
                (Task::none(), None)
            }
            LoginMessage::SelectServer(server) => {
                if let AuthPage::LoginEmail {
                    selected_server,
                    server_selector_open,
                    ..
                } = &mut self.auth_page
                {
                    *selected_server = server;
                    *server_selector_open = false;
                }
                (Task::none(), None)
            }

            // ── Account switcher ───────────────────────────────────────────
            LoginMessage::AccountSwitcher(asm) => match asm {
                AccountSwitcherMessage::ToggleDropdown => {
                    self.account_switcher_open = !self.account_switcher_open;
                    (Task::none(), None)
                }
                AccountSwitcherMessage::SwitchUser(uid) => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(LoginEvent::UserSelected { uid }))
                }
                AccountSwitcherMessage::AddAccount => {
                    self.account_switcher_open = false;
                    self.reset_to_email_entry();
                    (Task::none(), None)
                }
            },
        }
    }

    /// LOAD-BEARING: called from the App router before it dispatches a
    /// `Message::TitleBar(_)` to close any open login-side overlays so
    /// menu clicks don't leave them hanging. See the cross-view dismissal
    /// block in `App::update`.
    pub fn dismiss_dropdowns(&mut self) {
        self.account_switcher_open = false;
        if let AuthPage::LoginEmail {
            server_selector_open,
            ..
        } = &mut self.auth_page
        {
            *server_selector_open = false;
        }
    }

    /// Reset the login flow to the email-entry page with empty inputs.
    /// Called when the user picks "Add account" from either the login
    /// screen (locally, via `AccountSwitcherMessage::AddAccount`) or the
    /// vault screen (via `VaultEvent::AddAccountRequested` → handler).
    pub fn reset_to_email_entry(&mut self) {
        self.auth_page = AuthPage::new_login_email();
        self.unlock_in_progress = false;
    }

    /// Show the unlock page for the given user, picking their preferred
    /// unlock method. Falls back to master password when the user is absent
    /// or unknown to the client manager (e.g. during initial startup).
    /// Called from the post-load transition, user-switch, and lock-all flows.
    pub fn show_unlock_for(&mut self, uid: Option<&UserId>, mgr: &ClientManager) {
        let preferred = uid
            .and_then(|uid| mgr.unlock_methods(uid))
            .map(|m| m.preferred())
            .unwrap_or(UnlockMethod::MasterPassword);
        self.auth_page = AuthPage::new_unlock(preferred);
        self.unlock_in_progress = false;
    }

    pub fn view<'a>(
        &'a self,
        email: Option<&'a str>,
        server: &'a str,
        accounts: &'a [AccountEntry],
        unlock_alternatives: &'a [UnlockMethod],
        colors: &'a AppColors,
    ) -> Element<'a, LoginMessage, AppTheme> {
        let (center_content, status_bar) = match &self.auth_page {
            AuthPage::Unlock {
                method,
                password_input,
                pin_input,
            } => {
                let center = unlock::view(
                    *method,
                    unlock_alternatives,
                    email,
                    password_input,
                    pin_input,
                    self.unlock_in_progress,
                    colors,
                );
                let status = server_selector::simple_status(server, colors);
                (center, status)
            }
            AuthPage::LoginEmail {
                email_input,
                remember_email,
                server_selector_open,
                selected_server,
            } => {
                let center = login_email::view(email_input, *remember_email, colors);
                let status = server_selector::view(selected_server, *server_selector_open, colors);
                (center, status)
            }
            AuthPage::LoginPassword {
                email,
                password_input,
                selected_server,
            } => {
                let center = login_password::view(email, password_input, colors);
                let status =
                    server_selector::simple_status(&selected_server.display_name(), colors);
                (center, status)
            }
        };

        layout::auth_page_shell(
            center_content,
            status_bar,
            email,
            accounts,
            self.account_switcher_open,
            colors,
        )
    }
}
