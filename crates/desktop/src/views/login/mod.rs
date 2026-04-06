mod input_field;
mod layout;
mod login_email;
mod login_password;
mod server_selector;
mod unlock;

use iced::Element;

use crate::{
    components::account_switcher::{AccountEntry, AccountSwitcherMessage},
    state::UnlockMethod,
    theme::{AppColors, AppTheme},
};

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AuthPage {
    Unlock(UnlockMethod),
    LoginEmail,
    LoginPassword,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerOption {
    Bitwarden,
    BitwardenEu,
    SelfHosted(String),
}

impl ServerOption {
    pub fn display_name(&self) -> &str {
        match self {
            ServerOption::Bitwarden => "bitwarden.com",
            ServerOption::BitwardenEu => "bitwarden.eu",
            ServerOption::SelfHosted(url) if url.is_empty() => "Self-hosted",
            ServerOption::SelfHosted(url) => url.as_str(),
        }
    }
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LoginMessage {
    // Unlock — master password
    PasswordChanged(String),
    TogglePasswordVisibility,
    Unlock,
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
    ToggleLoginPasswordVisibility,
    LoginWithPassword,
    BackToEmail,
    GetPasswordHint,

    // Server selector
    ToggleServerSelector,
    SelectServer(ServerOption),

    // Account switcher
    AccountSwitcher(AccountSwitcherMessage),
}

// ── Actions ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LoginAction {
    Unlock(String),
    UnlockWithPin,
    UnlockWithBiometrics,
    LogOut,
    SwitchUser(String),
    Login { email: String, password: String },
    NavigateToAddAccount,
}

// ── View State ─────────────────────────────────────────────────────────────

pub struct LoginView {
    pub auth_page: AuthPage,
    // Unlock state
    pub password_input: String,
    pub pin_input: String,
    pub show_password: bool,
    // Login email state
    pub email_input: String,
    pub remember_email: bool,
    // Login password state
    pub login_password_input: String,
    pub show_login_password: bool,
    pub login_email: String,
    // Server selector
    pub server_selector_open: bool,
    pub selected_server: ServerOption,
    // Account switcher
    pub dropdown_open: bool,
}

impl LoginView {
    pub fn new() -> Self {
        Self {
            auth_page: AuthPage::Unlock(UnlockMethod::MasterPassword),
            password_input: String::new(),
            pin_input: String::new(),
            show_password: false,
            email_input: String::new(),
            remember_email: false,
            login_password_input: String::new(),
            show_login_password: false,
            login_email: String::new(),
            server_selector_open: false,
            selected_server: ServerOption::Bitwarden,
            dropdown_open: false,
        }
    }

    pub fn update(&mut self, msg: LoginMessage) -> Vec<LoginAction> {
        let mut actions = Vec::new();
        match msg {
            // ── Unlock: master password ────────────────────────────────────
            LoginMessage::PasswordChanged(pw) => self.password_input = pw,
            LoginMessage::TogglePasswordVisibility => {
                self.show_password = !self.show_password;
            }
            LoginMessage::Unlock => {
                let password = std::mem::take(&mut self.password_input);
                self.show_password = false;
                actions.push(LoginAction::Unlock(password));
            }

            // ── Unlock: PIN ────────────────────────────────────────────────
            LoginMessage::PinChanged(pin) => self.pin_input = pin,
            LoginMessage::UnlockWithPin => {
                self.pin_input.clear();
                actions.push(LoginAction::UnlockWithPin);
            }

            // ── Unlock: biometrics ─────────────────────────────────────────
            LoginMessage::UnlockWithBiometrics => {
                actions.push(LoginAction::UnlockWithBiometrics);
            }

            // ── Switch unlock method ───────────────────────────────────────
            LoginMessage::SwitchUnlockMethod(method) => {
                self.auth_page = AuthPage::Unlock(method);
                self.password_input.clear();
                self.pin_input.clear();
                self.show_password = false;
            }

            LoginMessage::LogOut => {
                actions.push(LoginAction::LogOut);
            }

            // ── Login: email entry ─────────────────────────────────────────
            LoginMessage::EmailChanged(email) => self.email_input = email,
            LoginMessage::ToggleRememberEmail(checked) => self.remember_email = checked,
            LoginMessage::ContinueWithEmail => {
                self.login_email = self.email_input.clone();
                self.auth_page = AuthPage::LoginPassword;
                self.login_password_input.clear();
                self.show_login_password = false;
            }
            LoginMessage::UseSingleSignOn => {
                // TODO: SSO login flow
            }

            // ── Login: password entry ──────────────────────────────────────
            LoginMessage::LoginPasswordChanged(pw) => self.login_password_input = pw,
            LoginMessage::ToggleLoginPasswordVisibility => {
                self.show_login_password = !self.show_login_password;
            }
            LoginMessage::LoginWithPassword => {
                let email = self.login_email.clone();
                let password = self.login_password_input.clone();
                self.login_password_input.clear();
                actions.push(LoginAction::Login { email, password });
            }
            LoginMessage::BackToEmail => {
                self.auth_page = AuthPage::LoginEmail;
                self.login_password_input.clear();
            }
            LoginMessage::GetPasswordHint => {
                // TODO: password hint request flow
            }

            // ── Server selector ────────────────────────────────────────────
            LoginMessage::ToggleServerSelector => {
                self.server_selector_open = !self.server_selector_open;
            }
            LoginMessage::SelectServer(server) => {
                self.selected_server = server;
                self.server_selector_open = false;
            }

            // ── Account switcher ───────────────────────────────────────────
            LoginMessage::AccountSwitcher(asm) => match asm {
                AccountSwitcherMessage::ToggleDropdown => {
                    self.dropdown_open = !self.dropdown_open;
                }
                AccountSwitcherMessage::SwitchUser(uid) => {
                    self.dropdown_open = false;
                    actions.push(LoginAction::SwitchUser(uid));
                }
                AccountSwitcherMessage::AddAccount => {
                    self.dropdown_open = false;
                    actions.push(LoginAction::NavigateToAddAccount);
                }
            },
        }
        actions
    }
}

// ── Top-level view dispatcher ──────────────────────────────────────────────

pub fn view<'a>(
    login_view: &'a LoginView,
    email: &'a str,
    server: &'a str,
    accounts: &'a [AccountEntry],
    unlock_alternatives: &'a [UnlockMethod],
    colors: &'a AppColors,
) -> Element<'a, LoginMessage, AppTheme> {
    let (center_content, status_bar) = match &login_view.auth_page {
        AuthPage::Unlock(method) => {
            let center = unlock::view(
                *method,
                unlock_alternatives,
                email,
                &login_view.password_input,
                &login_view.pin_input,
                login_view.show_password,
                colors,
            );
            let status = server_selector::simple_status(server, colors);
            (center, status)
        }
        AuthPage::LoginEmail => {
            let center = login_email::view(
                &login_view.email_input,
                login_view.remember_email,
                colors,
            );
            let status = server_selector::view(
                &login_view.selected_server,
                login_view.server_selector_open,
                colors,
            );
            (center, status)
        }
        AuthPage::LoginPassword => {
            let center = login_password::view(
                &login_view.login_email,
                &login_view.login_password_input,
                login_view.show_login_password,
                colors,
            );
            let status = server_selector::simple_status(
                login_view.selected_server.display_name(),
                colors,
            );
            (center, status)
        }
    };

    layout::auth_page_shell(
        center_content,
        status_bar,
        email,
        accounts,
        login_view.dropdown_open,
        colors,
    )
}
