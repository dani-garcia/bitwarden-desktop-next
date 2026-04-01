#[cfg(target_os = "macos")]
use muda::{Menu, MenuItem as MudaMenuItem, PredefinedMenuItem, Submenu};

// ---------------------------------------------------------------------------
// Shortcut helpers
// ---------------------------------------------------------------------------

/// Shortcut with Ctrl on Windows/Linux, Cmd on macOS.
const fn ck(key: &'static str, mac: &'static str) -> Shortcut {
    Shortcut { win: key, mac }
}

/// Same shortcut string on all platforms (e.g. "F11").
const fn same(key: &'static str) -> Shortcut {
    Shortcut { win: key, mac: key }
}

#[derive(Clone, Copy)]
pub struct Shortcut {
    win: &'static str,
    mac: &'static str,
}

impl Shortcut {
    pub fn text(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            self.mac
        } else {
            self.win
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
// Menu item types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
#[allow(dead_code)] // SubMenu items field used by macOS native menu, sub_items() for future drawn submenus
pub enum MenuEntry {
    Item {
        label: &'static str,
        shortcut: Option<Shortcut>,
        enabled: EnabledWhen,
    },
    Separator,
    SubMenu {
        label: &'static str,
        items: &'static [MenuEntry],
        enabled: EnabledWhen,
    },
}

impl MenuEntry {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Item { label, .. } | Self::SubMenu { label, .. } => label,
            Self::Separator => "",
        }
    }

    pub fn shortcut_text(&self) -> &'static str {
        match self {
            Self::Item { shortcut: Some(s), .. } => s.text(),
            _ => "",
        }
    }

    pub fn is_separator(&self) -> bool {
        matches!(self, Self::Separator)
    }

    pub fn is_submenu(&self) -> bool {
        matches!(self, Self::SubMenu { .. })
    }

    pub fn is_enabled(&self, state: &MenuState) -> bool {
        match self {
            Self::Item { enabled, .. } | Self::SubMenu { enabled, .. } => enabled.check(state),
            Self::Separator => true,
        }
    }

    #[allow(dead_code)] // Will be used when drawn submenus are implemented
    pub fn sub_items(&self) -> &'static [MenuEntry] {
        match self {
            Self::SubMenu { items, .. } => items,
            _ => &[],
        }
    }
}

// Shorthand constructors
const fn item(label: &'static str, enabled: EnabledWhen) -> MenuEntry {
    MenuEntry::Item { label, shortcut: None, enabled }
}

const fn item_s(label: &'static str, shortcut: Shortcut, enabled: EnabledWhen) -> MenuEntry {
    MenuEntry::Item { label, shortcut: Some(shortcut), enabled }
}

const fn sub(label: &'static str, items: &'static [MenuEntry], enabled: EnabledWhen) -> MenuEntry {
    MenuEntry::SubMenu { label, items, enabled }
}

const S: MenuEntry = MenuEntry::Separator;

use EnabledWhen::*;

// ---------------------------------------------------------------------------
// Menu definitions
// ---------------------------------------------------------------------------

pub const MENUS: &[(&str, &[MenuEntry])] = &[
    ("File", &[
        item_s("New login",        ck("Ctrl+N", "Cmd+N"),              Unlocked),
        sub("New item", &[
            item_s("Login",        ck("Ctrl+Shift+L", "Cmd+Shift+L"),  Always),
            item_s("Card",         ck("Ctrl+Shift+C", "Cmd+Shift+C"),  Always),
            item_s("Identity",     ck("Ctrl+Shift+I", "Cmd+Shift+I"),  Always),
            item_s("Secure note",  ck("Ctrl+Shift+S", "Cmd+Shift+S"),  Always),
            item_s("SSH key",      ck("Ctrl+Shift+K", "Cmd+Shift+K"),  Always),
        ], Unlocked),
        item("New folder",                                             Unlocked),
        S,
        item("Sync now",                                               HasAuthenticated),
        item("Import",                                                 Unlocked),
        item("Export",                                                 Unlocked),
        S,
        item_s("Settings",         ck("Ctrl+,", "Cmd+,"),              Unlocked),
        // Lock/Log out submenus: dynamically populated with account emails at runtime
        sub("Lock vault", &[],                                         HasLockable),
        item_s("Lock all vaults",  ck("Ctrl+L", "Cmd+L"),              HasAccounts),
        sub("Log out", &[],                                            HasAccounts),
        S,
        item("Quit Bitwarden",                                         Always),
    ]),
    ("Edit", &[
        item_s("Undo",                              ck("Ctrl+Z", "Cmd+Z"),           Always),
        item_s("Redo",                              ck("Ctrl+Y", "Cmd+Shift+Z"),     Always),
        S,
        item_s("Cut",                               ck("Ctrl+X", "Cmd+X"),           Always),
        item_s("Copy",                              ck("Ctrl+C", "Cmd+C"),           Always),
        item_s("Paste",                             ck("Ctrl+V", "Cmd+V"),           Always),
        S,
        item_s("Select all",                        ck("Ctrl+A", "Cmd+A"),           Always),
        S,
        item_s("Copy username",                     ck("Ctrl+U", "Cmd+U"),           Unlocked),
        item_s("Copy password",                     ck("Ctrl+P", "Cmd+P"),           Unlocked),
        item_s("Copy verification code (TOTP)",     ck("Ctrl+T", "Cmd+T"),           Unlocked),
    ]),
    ("View", &[
        item_s("Search vault",     ck("Ctrl+F", "Cmd+F"),              Unlocked),
        S,
        item_s("Generator",        ck("Ctrl+G", "Cmd+G"),              Unlocked),
        item("Generator history",                                      Unlocked),
        S,
        item_s("Zoom in",          ck("Ctrl++", "Cmd++"),              Always),
        item_s("Zoom out",         ck("Ctrl+-", "Cmd+-"),              Always),
        item_s("Reset zoom",       ck("Ctrl+0", "Cmd+0"),             Always),
        S,
        item_s("Toggle full screen", same("F11"),                      Always),
        S,
        item_s("Reload",           ck("Ctrl+Shift+R", "Cmd+Shift+R"), Always),
    ]),
    ("Account", &[
        item("Premium membership",                                     Unlocked),
        item("Change master password",                                 Unlocked),
        item("Two-step login",                                         Unlocked),
        item("Fingerprint phrase",                                     Unlocked),
        S,
        item("Delete account",                                         Unlocked),
    ]),
    ("Window", &[
        item_s("Minimize",         ck("Ctrl+M", "Cmd+M"),              Always),
        item_s("Hide to tray",     ck("Ctrl+Shift+M", "Cmd+Shift+M"), Always),
        item_s("Always on top",    ck("Ctrl+Shift+T", "Cmd+Shift+T"), Always),
        S,
        item_s("Close",            ck("Ctrl+W", "Cmd+W"),              Always),
    ]),
    ("Help", &[
        item("Help & feedback",                                        Always),
        item("File a bug report",                                      Always),
        sub("Legal", &[
            item("Terms of service",                                   Always),
            item("Privacy policy",                                     Always),
        ], Always),
        S,
        sub("Follow us", &[
            item("Blog",                                               Always),
            item("Twitter",                                            Always),
            item("Facebook",                                           Always),
            item("GitHub",                                             Always),
            item("Mastodon",                                           Always),
        ], Always),
        S,
        item("Go to web vault",                                        Always),
        S,
        sub("Get mobile app", &[
            item("iOS",                                                Always),
            item("Android",                                            Always),
        ], Always),
        sub("Get browser extension", &[
            item("Chrome",                                             Always),
            item("Firefox",                                            Always),
            item("Opera",                                              Always),
            item("Edge",                                               Always),
            item("Safari",                                             Always),
        ], Always),
        S,
        sub("Troubleshooting", &[
            item("Toggle hardware acceleration",                       Always),
        ], Always),
        S,
        item("About Bitwarden",                                        Always),
    ]),
];

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
            match entry {
                MenuEntry::Separator => Box::new(PredefinedMenuItem::separator()),
                MenuEntry::Item { label, .. } => {
                    Box::new(MudaMenuItem::new(*label, true, None))
                }
                MenuEntry::SubMenu { label, items, .. } => {
                    let sub = Submenu::new(*label, true);
                    append_entries_to_submenu(&sub, items);
                    Box::new(sub)
                }
            }
        })
        .collect();
    let refs: Vec<&dyn muda::IsMenuItem> = items.iter().map(|b| b.as_ref()).collect();
    let _ = submenu.append_items(&refs);
}
