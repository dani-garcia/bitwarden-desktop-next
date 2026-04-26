//! Settings modal — app-wide and per-user preferences organized into five
//! tabs (Security, Integrations, Autotype & copy, Appearance, Advanced).
//!
//! Opens via the File → Settings menu entry (Ctrl/Cmd+,), available only when
//! an account is unlocked. Closes via the X button, backdrop click, or Escape.
//!
//! Only a handful of fields are live-wired this pass — theme, language, tray
//! options, and the per-user clear-clipboard delay. Everything else emits a
//! `settings-toast-not-supported` warning toast so the UI can be fleshed out
//! ahead of the underlying SDK / OS wiring.

mod handler;
mod tabs;

use iced::{
    Alignment, Border, Element, Fill, Length, Padding,
    border::Radius,
    widget::{Space, column, container, row, scrollable, text},
};

use crate::{
    app::{Outcome, ViewTypes},
    components::{self, buttons, icons, modal, toast::Toast},
    fl,
    services::{
        preferences::{DurationSecs, SshPromptBehavior, UserPreferences},
        settings::Settings,
    },
    theme::{AppColors, AppTheme, RADIUS_LG, RADIUS_MD, ThemePreference},
};

// ── State ──────────────────────────────────────────────────────────────────

pub struct SettingsView {
    pub fade: components::FadeInOut,
    active: CategoryKind,
    /// Working copy of `(Settings, UserPreferences)`. Every edit mutates this
    /// directly and also bubbles up via `SettingsEvent::Applied` so App can
    /// copy it back into its persisted state and run any live side effect.
    /// Tabs render straight out of this snapshot — no intermediate form layer.
    pub snapshot: SettingsSnapshot,
}

impl ViewTypes for SettingsView {
    type Message = SettingsMessage;
    type Event = SettingsEvent;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryKind {
    Security,
    Integrations,
    Autotype,
    Appearance,
    Advanced,
}

impl CategoryKind {
    const ALL: &'static [Self] = &[
        Self::Security,
        Self::Integrations,
        Self::Autotype,
        Self::Appearance,
        Self::Advanced,
    ];

    fn title(self) -> String {
        match self {
            Self::Security => fl!("settings-tab-security"),
            Self::Integrations => fl!("settings-tab-integrations"),
            Self::Autotype => fl!("settings-tab-autotype"),
            Self::Appearance => fl!("settings-tab-appearance"),
            Self::Advanced => fl!("settings-tab-advanced"),
        }
    }

    fn icon(self) -> icons::Icon {
        match self {
            Self::Security => icons::SHIELD,
            Self::Integrations => icons::DIAGRAM_3,
            Self::Autotype => icons::KEYBOARD,
            Self::Appearance => icons::BRUSH,
            Self::Advanced => icons::TOOLS,
        }
    }
}

/// Working `(Settings, UserPreferences)` pair the view owns while the modal
/// is open. Built by App on open and handed over; edits mutate it in-place
/// and also bubble out via [`SettingsEvent::Applied`] so App can live-wire
/// the side effect and persist.
#[derive(Debug, Clone, Default)]
pub struct SettingsSnapshot {
    pub settings: Settings,
    pub prefs: UserPreferences,
}

// ── Messages + Events ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    Close,
    SelectCategory(CategoryKind),
    SettingChanged(SettingChange),
}

/// Every per-field edit funnels through this enum. Keeps the top-level
/// message enum flat and gives the App router one place to decide whether a
/// change is live-wired or stub-toasted.
#[derive(Debug, Clone)]
pub enum SettingChange {
    // Security
    OpenAtLogin(bool),
    PinUnlock(bool),
    TouchIdUnlock(bool),
    LockAfter(DurationSecs),
    LogoutAfter(DurationSecs),
    // Integrations
    BrowserIntegration(bool),
    BrowserIntegrationFingerprint(bool),
    SshAgent(bool),
    SshPromptBehavior(SshPromptBehavior),
    DuckDuckGo(bool),
    // Autotype & copy
    AutotypeEnabled(bool),
    ClearClipboard(DurationSecs),
    MinimizeOnCopy(bool),
    // Appearance
    Theme(ThemePreference),
    Language(String),
    ShowFavicons(bool),
    // Advanced
    TrayEnabled(bool),
    MinimizeToTray(bool),
    CloseToTray(bool),
    AlwaysShowDock(bool),
    HardwareAcceleration(bool),
    AllowScreenshots(bool),
}

pub enum SettingsEvent {
    /// Apply a field change at the App level (which decides live vs stub).
    Applied(SettingChange),
}

// ── Lifecycle ──────────────────────────────────────────────────────────────

impl SettingsView {
    pub fn new() -> Self {
        Self {
            fade: components::FadeInOut::default(),
            active: CategoryKind::Security,
            snapshot: SettingsSnapshot::default(),
        }
    }

    /// True while the user has the settings modal logically open. Distinct
    /// from "currently rendered" — the outro animation keeps the view alive
    /// for ~180ms after `close()` is called, but `is_open()` already returns
    /// false at that point so Escape-key handlers don't re-fire.
    pub fn is_open(&self) -> bool {
        self.fade.is_open()
    }

    pub fn open_with(&mut self, snap: SettingsSnapshot) {
        self.active = CategoryKind::Security;
        self.snapshot = snap;
        self.fade.open();
    }

    pub fn close(&mut self) {
        self.fade.close();
    }

    pub fn update(
        &mut self,
        msg: SettingsMessage,
        _ctx: crate::app::UpdateCtx<'_>,
    ) -> Outcome<Self> {
        match msg {
            SettingsMessage::Close => self.fade.close(),
            SettingsMessage::SelectCategory(kind) => self.active = kind,
            SettingsMessage::SettingChanged(change) => {
                self.apply_to_snapshot(&change);
                return Outcome::event(SettingsEvent::Applied(change));
            }
        }
        Outcome::None
    }

    /// Mirror the edit into the working snapshot so the widget re-renders
    /// with the new value on the next frame — independent of whether App
    /// live-wires the change or just toasts "not supported".
    fn apply_to_snapshot(&mut self, change: &SettingChange) {
        let s = &mut self.snapshot.settings;
        let p = &mut self.snapshot.prefs;
        match change {
            SettingChange::OpenAtLogin(v) => s.open_at_login = *v,
            SettingChange::PinUnlock(v) => p.pin_unlock = *v,
            SettingChange::TouchIdUnlock(v) => p.touch_id_unlock = *v,
            SettingChange::LockAfter(v) => p.lock_after = *v,
            SettingChange::LogoutAfter(v) => p.logout_after = *v,
            SettingChange::BrowserIntegration(v) => s.browser_integration = *v,
            SettingChange::BrowserIntegrationFingerprint(v) => {
                s.browser_integration_fingerprint = *v
            }
            SettingChange::SshAgent(v) => s.ssh_agent = *v,
            SettingChange::SshPromptBehavior(v) => p.ssh_prompt_behavior = *v,
            SettingChange::DuckDuckGo(v) => s.duck_duck_go = *v,
            SettingChange::AutotypeEnabled(v) => s.autotype_enabled = *v,
            SettingChange::ClearClipboard(v) => p.clear_clipboard = *v,
            SettingChange::MinimizeOnCopy(v) => p.minimize_on_copy = *v,
            SettingChange::Theme(v) => s.theme = *v,
            SettingChange::Language(v) => s.language = v.clone(),
            SettingChange::ShowFavicons(v) => s.show_favicons = *v,
            SettingChange::TrayEnabled(v) => s.show_tray_icon = *v,
            SettingChange::MinimizeToTray(v) => s.minimize_to_tray = *v,
            SettingChange::CloseToTray(v) => s.close_to_tray = *v,
            SettingChange::AlwaysShowDock(v) => s.always_show_dock = *v,
            SettingChange::HardwareAcceleration(v) => s.hardware_acceleration = *v,
            SettingChange::AllowScreenshots(v) => s.allow_screenshots = *v,
        }
    }

    /// Returns `None` when the modal is closed — App composes into the
    /// overlay stack only when this yields `Some(_)`.
    pub fn modal_view<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Option<Element<'a, SettingsMessage, AppTheme>> {
        let progress = self.fade.progress_if_visible()?;

        let sidebar = self.sidebar_view(colors);
        let pane = self.content_pane(colors);

        // Outer dialog: fixed-size, white bg, rounded all corners. The sidebar
        // paints its own light-gray background with matching LEFT corner radii
        // so its fill aligns with the dialog's rounded edge instead of masking
        // it (CLAUDE.md → "Nested container backgrounds mask parent border-radius").
        let body = row![sidebar, pane].width(Fill).height(Fill);

        Some(modal::dialog(
            640.0,
            Some(440.0),
            |c| c.background,
            progress,
            body.into(),
            SettingsMessage::Close,
        ))
    }

    fn sidebar_view<'a>(&'a self, colors: &'a AppColors) -> Element<'a, SettingsMessage, AppTheme> {
        let mut items = column![
            text(fl!("settings-title"))
                .size(28)
                .font(crate::APP_FONT_BOLD)
                .color(colors.text_primary),
            Space::new().height(16),
        ]
        .spacing(4);

        for &kind in CategoryKind::ALL {
            items = items.push(category_item(kind, self.active == kind, colors));
        }

        container(items)
            .padding(Padding::from([20, 16]))
            .width(Length::Fixed(200.0))
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(theme.colors.card_bg)
                    .border(Border::default().rounded(Radius {
                        top_left: RADIUS_LG,
                        top_right: 0.0,
                        bottom_left: RADIUS_LG,
                        bottom_right: 0.0,
                    }))
            })
            .into()
    }

    fn content_pane<'a>(&'a self, colors: &'a AppColors) -> Element<'a, SettingsMessage, AppTheme> {
        let header = row![
            text(self.active.title())
                .size(20)
                .font(crate::APP_FONT_BOLD)
                .color(colors.text_primary),
            Space::new().width(Fill),
            buttons::ghost_icon(
                icons::X_LG.render(16.0, colors.text_primary),
                colors.item_hover,
            )
            .padding([6, 6])
            .on_press(SettingsMessage::Close),
        ]
        .align_y(Alignment::Center);

        let body = match self.active {
            CategoryKind::Security => tabs::security::view(&self.snapshot, colors),
            CategoryKind::Integrations => tabs::integrations::view(&self.snapshot, colors),
            CategoryKind::Autotype => tabs::autotype::view(&self.snapshot, colors),
            CategoryKind::Appearance => tabs::appearance::view(&self.snapshot, colors),
            CategoryKind::Advanced => tabs::advanced::view(&self.snapshot, colors),
        };

        let content = column![
            header,
            Space::new().height(20),
            scrollable(body.map(SettingsMessage::SettingChanged)).height(Fill),
        ]
        .width(Fill)
        .height(Fill);

        container(content)
            .padding(Padding::from([20, 24]))
            .width(Fill)
            .height(Fill)
            .into()
    }
}

impl Default for SettingsView {
    fn default() -> Self {
        Self::new()
    }
}

/// Sidebar row: icon + label, highlighted when it's the active category.
fn category_item<'a>(
    kind: CategoryKind,
    active: bool,
    colors: &'a AppColors,
) -> Element<'a, SettingsMessage, AppTheme> {
    let icon_color = if active {
        colors.accent
    } else {
        colors.text_primary
    };
    let text_color = icon_color;

    let content = row![
        kind.icon().render(16.0, icon_color),
        text(kind.title()).size(14).color(text_color),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let padded = container(content)
        .padding(Padding::from([8, 10]))
        .width(Fill);

    buttons::ghost(
        padded,
        active,
        colors.surface_selected,
        colors.item_hover,
        RADIUS_MD,
    )
    .width(Fill)
    .on_press(SettingsMessage::SelectCategory(kind))
    .into()
}

// ── Small helpers shared by tabs ───────────────────────────────────────────

pub(super) fn section_heading<'a>(
    label: impl Into<String>,
    colors: &'a AppColors,
) -> Element<'a, SettingChange, AppTheme> {
    text(label.into())
        .size(16)
        .font(crate::APP_FONT_BOLD)
        .color(colors.text_primary)
        .into()
}

/// Placeholder toast for stubbed settings. Exposed so App can construct the
/// same toast when it receives a stubbed `Applied` event.
pub fn not_supported_toast() -> Toast {
    Toast::warning(fl!("settings-toast-not-supported"), None)
}
