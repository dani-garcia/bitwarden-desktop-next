use iced::Task;

use crate::{
    app::{App, Message},
    views::settings::{SettingChange, SettingsEvent},
};

impl App {
    pub(crate) fn handle_settings_event(&mut self, event: SettingsEvent) -> Task<Message> {
        match event {
            SettingsEvent::Applied(change) => self.apply_setting_change(change),
        }
    }

    /// Pull the view's working snapshot back into the persisted
    /// [`Settings`][crate::services::settings::Settings] (the view is the source of
    /// truth during the modal's lifetime), then run any runtime side effect
    /// that the change requires — a theme refresh, a language switch,
    /// clipboard-timeout push, tray respawn — or emit the generic
    /// "not supported yet" toast for stubbed fields.
    fn apply_setting_change(&mut self, change: SettingChange) -> Task<Message> {
        // Copy every field the view just mutated back into App state. The
        // view's snapshot holds the fresh `Settings` (including the full
        // `user_preferences` map from open time) plus the active user's
        // freshly-edited `UserPreferences`; overwrite the active slot so
        // other users' prefs stay intact.
        self.settings = self.views.settings.snapshot.settings.clone();
        if let Some(uid) = self.active_user {
            self.settings
                .user_preferences
                .insert(uid, self.views.settings.snapshot.prefs);
        }
        self.settings.save();

        // Side effects: live-wired changes need a nudge, stubs get a toast.
        match change {
            SettingChange::Theme(p) => {
                self.theme.preference = p;
                self.theme.current = p.resolve(self.theme.system.as_deref());
            }
            SettingChange::Language(tag) => {
                if tag.is_empty() {
                    // Empty = follow OS locale. Re-run the initial selection
                    // so the next `fl!()` call picks up the OS preference.
                    crate::services::i18n::init();
                } else if let Ok(lang_id) = tag.parse() {
                    crate::services::i18n::set_language(lang_id);
                } else {
                    tracing::warn!(%tag, "unparseable language tag; ignoring");
                }
            }
            SettingChange::ClearClipboard(delay) => {
                self.clipboard.set_timeout(delay.as_duration());
            }
            SettingChange::TrayEnabled(_)
            | SettingChange::MinimizeToTray(_)
            | SettingChange::CloseToTray(_) => self.refresh_tray(),

            // No runtime side-effect needed — the setting is read where it
            // takes effect. `MinimizeOnCopy` is consumed in the vault
            // clipboard handler; `ShowFavicons` is consumed by the item-list
            // renderer; `HardwareAcceleration` is read once at startup (see
            // `main.rs`) and only takes effect on restart;
            // `LockOnSystemLock` is read by the session-event handler in
            // `app/handlers/platform.rs` when an event fires.
            SettingChange::MinimizeOnCopy(_)
            | SettingChange::ShowFavicons(_)
            | SettingChange::HardwareAcceleration(_)
            | SettingChange::LockOnSystemLock(_) => {}

            // Both timeouts are consumed by `services::session_timeout`'s
            // deadline computation — bump it now so the new value takes
            // effect without waiting for the next event.
            SettingChange::LockAfter(_) | SettingChange::LogoutAfter(_) => {
                self.refresh_session_timeout_deadline();
            }

            // Every remaining variant is currently unwired — the value was
            // persisted above, but the feature doesn't react yet. Let the
            // user know with a toast.
            SettingChange::OpenAtLogin(_)
            | SettingChange::PinUnlock(_)
            | SettingChange::TouchIdUnlock(_)
            | SettingChange::BrowserIntegration(_)
            | SettingChange::BrowserIntegrationFingerprint(_)
            | SettingChange::SshAgent(_)
            | SettingChange::SshPromptBehavior(_)
            | SettingChange::DuckDuckGo(_)
            | SettingChange::AutotypeEnabled(_)
            | SettingChange::AlwaysShowDock(_)
            | SettingChange::AllowScreenshots(_) => {
                self.push_toast(crate::views::settings::not_supported_toast());
            }
        }
        Task::none()
    }

    /// Ensure the tray reflects `self.settings.wants_tray()` — create one on
    /// enable, drop it on full disable. Called after any tray-related
    /// setting change.
    fn refresh_tray(&mut self) {
        let wants = self.settings.wants_tray();
        match (wants, self.tray.is_some()) {
            (true, false) => {
                self.tray = crate::services::tray::build();
                if self.tray.is_none() {
                    tracing::warn!("tray requested via settings but failed to initialise");
                }
            }
            (false, true) => {
                self.tray = None;
            }
            _ => {}
        }
    }
}
