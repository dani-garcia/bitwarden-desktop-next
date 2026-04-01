#[cfg(target_os = "macos")]
use muda::{Menu, MenuItem as MudaMenuItem, PredefinedMenuItem, Submenu};

// ---------------------------------------------------------------------------
// Shortcut
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    Char(char), // lowercase letter or symbol
    F(u8),      // F1..F12
}

#[derive(Clone, Copy)]
pub struct Shortcut {
    pub ctrl_cmd: bool, // Ctrl on Win/Linux, Cmd on macOS
    pub shift: bool,
    pub key: ShortcutKey,
}

/// Ctrl/Cmd + key
const fn cmd(key: char) -> Shortcut {
    Shortcut { ctrl_cmd: true, shift: false, key: ShortcutKey::Char(key) }
}

/// Ctrl/Cmd + Shift + key
const fn cmd_shift(key: char) -> Shortcut {
    Shortcut { ctrl_cmd: true, shift: true, key: ShortcutKey::Char(key) }
}

/// Function key alone
const fn fkey(n: u8) -> Shortcut {
    Shortcut { ctrl_cmd: false, shift: false, key: ShortcutKey::F(n) }
}

impl Shortcut {
    /// Platform-aware display text (e.g. "Ctrl+N" on Win, "Cmd+N" on Mac).
    pub fn display(&self) -> String {
        let mut s = String::new();
        if cfg!(target_os = "macos") {
            if self.ctrl_cmd { s.push_str("Cmd+"); }
            if self.shift { s.push_str("Shift+"); }
        } else {
            if self.ctrl_cmd { s.push_str("Ctrl+"); }
            if self.shift { s.push_str("Shift+"); }
        }
        match self.key {
            ShortcutKey::Char(c) if c.is_ascii_alphabetic() => s.push(c.to_ascii_uppercase()),
            ShortcutKey::Char(c) => s.push(c),
            ShortcutKey::F(n) => { s.push('F'); s.push_str(&n.to_string()); }
        }
        s
    }

    /// Returns true if an iced keyboard event matches this shortcut.
    pub fn matches(
        &self,
        key: &iced::keyboard::Key,
        modifiers: iced::keyboard::Modifiers,
    ) -> bool {
        if modifiers.command() != self.ctrl_cmd { return false; }
        if modifiers.shift() != self.shift { return false; }

        match (&self.key, key) {
            (ShortcutKey::Char(c), iced::keyboard::Key::Character(s)) => {
                s.chars().next()
                    .is_some_and(|pressed| pressed.eq_ignore_ascii_case(c))
            }
            (ShortcutKey::F(n), iced::keyboard::Key::Named(named)) => {
                use iced::keyboard::key::Named::*;
                let matched = match named {
                    F1 => 1, F2 => 2, F3 => 3, F4 => 4, F5 => 5, F6 => 6,
                    F7 => 7, F8 => 8, F9 => 9, F10 => 10, F11 => 11, F12 => 12,
                    _ => return false,
                };
                *n == matched
            }
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Enabled conditions
// ---------------------------------------------------------------------------

/// Runtime state that determines which menu items are enabled.
#[derive(Clone, Debug)]
pub struct MenuState {
    pub is_locked: bool,
    pub has_accounts: bool,
    pub has_lockable_accounts: bool,
    pub has_authenticated_accounts: bool,
}

impl Default for MenuState {
    fn default() -> Self {
        Self {
            is_locked: true,
            has_accounts: false,
            has_lockable_accounts: false,
            has_authenticated_accounts: false,
        }
    }
}

/// Condition that determines whether a menu item is enabled.
#[derive(Clone, Copy)]
pub enum EnabledWhen {
    Always,
    Unlocked,
    HasAccounts,
    HasLockable,
    HasAuthenticated,
}

impl EnabledWhen {
    pub fn check(self, state: &MenuState) -> bool {
        match self {
            Self::Always => true,
            Self::Unlocked => !state.is_locked,
            Self::HasAccounts => state.has_accounts,
            Self::HasLockable => state.has_lockable_accounts,
            Self::HasAuthenticated => state.has_authenticated_accounts,
        }
    }
}

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

/// Identifies an actionable menu command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Quit,
    LockAllVaults,
    SyncNow,
    SearchVault,
    ToggleFullScreen,
    Reload,
    Minimize,
    HideToTray,
    ToggleAlwaysOnTop,
    Close,
    About,
}

// ---------------------------------------------------------------------------
// MenuEntry (flat struct with builder)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct MenuEntry {
    pub label: &'static str,
    pub shortcut: Option<Shortcut>,
    pub enabled: EnabledWhen,
    pub action: Option<MenuAction>,
    pub children: &'static [MenuEntry],
}

/// Separator constant.
const SEP: MenuEntry = MenuEntry {
    label: "",
    shortcut: None,
    enabled: EnabledWhen::Always,
    action: None,
    children: &[],
};

/// Start building a menu entry.
#[allow(non_snake_case)]
const fn E(label: &'static str) -> MenuEntry {
    MenuEntry {
        label,
        shortcut: None,
        enabled: EnabledWhen::Always,
        action: None,
        children: &[],
    }
}

impl MenuEntry {
    const fn key(mut self, s: Shortcut) -> Self { self.shortcut = Some(s); self }
    const fn when(mut self, e: EnabledWhen) -> Self { self.enabled = e; self }
    const fn action(mut self, a: MenuAction) -> Self { self.action = Some(a); self }
    const fn sub(mut self, items: &'static [MenuEntry]) -> Self { self.children = items; self }

    pub fn is_separator(&self) -> bool { self.label.is_empty() && self.children.is_empty() }
    pub fn is_submenu(&self) -> bool { !self.children.is_empty() || self.is_submenu_placeholder() }
    pub fn is_enabled(&self, state: &MenuState) -> bool { self.enabled.check(state) }

    /// Submenus with empty children (e.g. "Lock vault", "Log out") are placeholders
    /// for dynamically populated content. They still render as submenus.
    fn is_submenu_placeholder(&self) -> bool {
        matches!(self.enabled, EnabledWhen::HasLockable | EnabledWhen::HasAccounts)
            && self.action.is_none()
            && self.shortcut.is_none()
            && self.children.is_empty()
            && !self.label.is_empty()
    }

    pub fn shortcut_display(&self) -> Option<String> {
        self.shortcut.map(|s| s.display())
    }
}

// ---------------------------------------------------------------------------
// Menu definitions
// ---------------------------------------------------------------------------

use EnabledWhen::*;
use MenuAction::*;

pub const MENUS: &[(&str, &[MenuEntry])] = &[
    ("File", &[
        E("New login").key(cmd('n')).when(Unlocked),
        E("New item").when(Unlocked).sub(&[
            E("Login").key(cmd_shift('l')),
            E("Card").key(cmd_shift('c')),
            E("Identity").key(cmd_shift('i')),
            E("Secure note").key(cmd_shift('s')),
            E("SSH key").key(cmd_shift('k')),
        ]),
        E("New folder").when(Unlocked),
        SEP,
        E("Sync now").when(HasAuthenticated).action(SyncNow),
        E("Import").when(Unlocked),
        E("Export").when(Unlocked),
        SEP,
        E("Settings").key(cmd(',')).when(Unlocked),
        // Lock/Log out submenus: dynamically populated with account emails at runtime
        E("Lock vault").when(HasLockable).sub(&[]),
        E("Lock all vaults").key(cmd('l')).when(HasAccounts).action(LockAllVaults),
        E("Log out").when(HasAccounts).sub(&[]),
        SEP,
        E("Quit Bitwarden").action(Quit),
    ]),
    ("Edit", &[
        E("Undo").key(cmd('z')),
        E("Redo").key(cmd('y')),
        SEP,
        E("Cut").key(cmd('x')),
        E("Copy").key(cmd('c')),
        E("Paste").key(cmd('v')),
        SEP,
        E("Select all").key(cmd('a')),
        SEP,
        E("Copy username").key(cmd('u')).when(Unlocked),
        E("Copy password").key(cmd('p')).when(Unlocked),
        E("Copy verification code (TOTP)").key(cmd('t')).when(Unlocked),
    ]),
    ("View", &[
        E("Search vault").key(cmd('f')).when(Unlocked).action(SearchVault),
        SEP,
        E("Generator").key(cmd('g')).when(Unlocked),
        E("Generator history").when(Unlocked),
        SEP,
        E("Zoom in").key(cmd('+')),
        E("Zoom out").key(cmd('-')),
        E("Reset zoom").key(cmd('0')),
        SEP,
        E("Toggle full screen").key(fkey(11)).action(ToggleFullScreen),
        SEP,
        E("Reload").key(cmd_shift('r')).action(Reload),
    ]),
    ("Account", &[
        E("Premium membership").when(Unlocked),
        E("Change master password").when(Unlocked),
        E("Two-step login").when(Unlocked),
        E("Fingerprint phrase").when(Unlocked),
        SEP,
        E("Delete account").when(Unlocked),
    ]),
    ("Window", &[
        E("Minimize").key(cmd('m')).action(Minimize),
        E("Hide to tray").key(cmd_shift('m')).action(HideToTray),
        E("Always on top").key(cmd_shift('t')).action(ToggleAlwaysOnTop),
        SEP,
        E("Close").key(cmd('w')).action(Close),
    ]),
    ("Help", &[
        E("Help & feedback"),
        E("File a bug report"),
        E("Legal").sub(&[
            E("Terms of service"),
            E("Privacy policy"),
        ]),
        SEP,
        E("Follow us").sub(&[
            E("Blog"),
            E("Twitter"),
            E("Facebook"),
            E("GitHub"),
            E("Mastodon"),
        ]),
        SEP,
        E("Go to web vault"),
        SEP,
        E("Get mobile app").sub(&[
            E("iOS"),
            E("Android"),
        ]),
        E("Get browser extension").sub(&[
            E("Chrome"),
            E("Firefox"),
            E("Opera"),
            E("Edge"),
            E("Safari"),
        ]),
        SEP,
        E("Troubleshooting").sub(&[
            E("Toggle hardware acceleration"),
        ]),
        SEP,
        E("About Bitwarden").action(About),
    ]),
];

// ---------------------------------------------------------------------------
// Shortcut → action lookup (for keyboard handling)
// ---------------------------------------------------------------------------

/// Find the MenuAction for a keyboard event, checking all menus and submenus.
pub fn find_shortcut_action(
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    state: &MenuState,
) -> Option<MenuAction> {
    for (_label, entries) in MENUS {
        if let Some(action) = find_in_entries(entries, key, modifiers, state) {
            return Some(action);
        }
    }
    None
}

fn find_in_entries(
    entries: &[MenuEntry],
    key: &iced::keyboard::Key,
    modifiers: iced::keyboard::Modifiers,
    state: &MenuState,
) -> Option<MenuAction> {
    for entry in entries {
        if let Some(s) = entry.shortcut
            && s.matches(key, modifiers)
            && entry.is_enabled(state)
            && let Some(action) = entry.action
        {
            return Some(action);
        }
        if !entry.children.is_empty()
            && let Some(action) = find_in_entries(entry.children, key, modifiers, state)
        {
            return Some(action);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Native menu (macOS only)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
pub fn attach_menu(_raw_id: u64) {
    let menu = build_menu();
    menu.init_for_nsapp();
    std::mem::forget(menu);
}

#[cfg(not(target_os = "macos"))]
pub fn attach_menu(_raw_id: u64) {}

pub const fn should_draw_menu() -> bool {
    !cfg!(target_os = "macos")
}

#[cfg(target_os = "macos")]
fn build_menu() -> Menu {
    let menu = Menu::new();
    for (label, entries) in MENUS {
        let submenu = Submenu::new(&format!("&{label}"), true);
        append_entries_to_submenu(&submenu, entries);
        let _ = menu.append(&submenu);
    }
    menu
}

#[cfg(target_os = "macos")]
fn append_entries_to_submenu(submenu: &Submenu, entries: &[MenuEntry]) {
    let items: Vec<Box<dyn muda::IsMenuItem>> = entries
        .iter()
        .map(|entry| -> Box<dyn muda::IsMenuItem> {
            if entry.is_separator() {
                Box::new(PredefinedMenuItem::separator())
            } else if !entry.children.is_empty() {
                let sub = Submenu::new(entry.label, true);
                append_entries_to_submenu(&sub, entry.children);
                Box::new(sub)
            } else {
                Box::new(MudaMenuItem::new(entry.label, true, None))
            }
        })
        .collect();
    let refs: Vec<&dyn muda::IsMenuItem> = items.iter().map(|b| b.as_ref()).collect();
    let _ = submenu.append_items(&refs);
}
