//! Per-entry data shared by the custom and native menu renderers.
//!
//! - [`MenuState`] / [`EnabledWhen`] — gate per-entry enabled status from the
//!   live App state (locked, has accounts, etc.)
//! - [`MenuAction`] — the verb the entry triggers when clicked.
//! - [`MenuEntry`] — flat struct + small const builder. The `MENUS` table
//!   in [`super::definitions`] composes these.

use bitwarden_vault::CipherType;

use super::shortcut::Shortcut;

#[derive(Clone, Debug)]
pub struct MenuState {
    pub is_locked: bool,
    pub has_accounts: bool,
    pub has_lockable_accounts: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            is_locked: true,
            has_accounts: false,
            has_lockable_accounts: false,
        }
    }
}

#[derive(Clone, Copy)]
pub enum EnabledWhen {
    Always,
    Unlocked,
    HasAccounts,
    HasLockable,
    /// Permanently disabled — entry renders grayed-out. Used as a placeholder
    /// for menu items that exist on the official client but have no behaviour
    /// in this stub (Edit → Undo / Redo / Cut / Copy / Paste / Select all).
    /// Keyboard shortcuts still pass through to focused text widgets
    /// unaffected, since they have no `MenuAction`.
    Never,
}

impl EnabledWhen {
    pub fn check(self, state: &MenuState) -> bool {
        match self {
            Self::Always => true,
            Self::Unlocked => !state.is_locked,
            Self::HasAccounts => state.has_accounts,
            Self::HasLockable => state.has_lockable_accounts,
            Self::Never => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuAction {
    Quit,
    LockAllVaults,
    SyncNow,
    SearchVault,
    ToggleFullScreen,
    Minimize,
    HideToTray,
    ToggleAlwaysOnTop,
    Close,
    About,
    Settings,
    Generator,
    GeneratorHistory,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleHardwareAcceleration,
    Import,
    Export,
    NewFolder,
    /// Open the vault with an empty cipher form of the given type.
    /// Wired from the File → New login (Cmd+N) entry, the File → New item
    /// submenu (Cmd+Shift+L/C/I/S/K), and the +New dropdown.
    NewItem(CipherType),
    CopyUsername,
    CopyPassword,
    CopyTotp,
    FingerprintPhrase,
    /// Static external URL — Help-menu social/store/legal links.
    OpenStaticUrl(&'static str),
    /// The active user's web vault. `None` opens the root; `Some("#/path")`
    /// appends an in-app route (account-menu items).
    OpenWebVault(Option<&'static str>),
}

#[derive(Clone, Copy)]
pub struct MenuEntry {
    pub label: &'static str,
    pub shortcut: Option<Shortcut>,
    pub enabled: EnabledWhen,
    pub action: Option<MenuAction>,
    pub children: &'static [MenuEntry],
    /// When `true`, `label` is rendered verbatim instead of being looked up
    /// as a Fluent key. Use `L()` instead of `E()` for brand names and
    /// platform names ("Chrome", "iOS", …) that shouldn't be translated.
    pub literal: bool,
}

pub(super) const SEP: MenuEntry = MenuEntry {
    label: "",
    shortcut: None,
    enabled: EnabledWhen::Always,
    action: None,
    children: &[],
    literal: false,
};

/// `label` is a Fluent message ID.
#[expect(non_snake_case)]
pub(super) const fn E(label: &'static str) -> MenuEntry {
    MenuEntry {
        label,
        shortcut: None,
        enabled: EnabledWhen::Always,
        action: None,
        children: &[],
        literal: false,
    }
}

/// **Literal** label — rendered verbatim, no Fluent lookup. Use for brand /
/// platform names that shouldn't be translated (Chrome, Firefox, iOS, …).
#[expect(non_snake_case)]
pub(super) const fn L(label: &'static str) -> MenuEntry {
    MenuEntry {
        label,
        shortcut: None,
        enabled: EnabledWhen::Always,
        action: None,
        children: &[],
        literal: true,
    }
}

impl MenuEntry {
    pub(super) const fn key(mut self, s: Shortcut) -> Self {
        self.shortcut = Some(s);
        self
    }
    pub(super) const fn when(mut self, e: EnabledWhen) -> Self {
        self.enabled = e;
        self
    }
    pub(super) const fn action(mut self, a: MenuAction) -> Self {
        self.action = Some(a);
        self
    }
    pub(super) const fn sub(mut self, items: &'static [MenuEntry]) -> Self {
        self.children = items;
        self
    }

    pub fn is_separator(&self) -> bool {
        self.label.is_empty() && self.children.is_empty()
    }
    pub fn is_submenu(&self) -> bool {
        !self.children.is_empty() || self.is_submenu_placeholder()
    }
    pub fn is_enabled(&self, state: &MenuState) -> bool {
        self.enabled.check(state)
    }

    /// Submenus with empty children (e.g. "Lock vault", "Log out") are placeholders
    /// for dynamically populated content. They still render as submenus.
    fn is_submenu_placeholder(&self) -> bool {
        matches!(
            self.enabled,
            EnabledWhen::HasLockable | EnabledWhen::HasAccounts
        ) && self.action.is_none()
            && self.shortcut.is_none()
            && self.children.is_empty()
            && !self.label.is_empty()
    }

    pub fn shortcut_display(&self) -> Option<String> {
        self.shortcut.map(|s| s.display())
    }

    pub fn display_label(&self) -> String {
        if self.literal {
            self.label.to_string()
        } else {
            crate::services::i18n::lookup(self.label)
        }
    }
}
