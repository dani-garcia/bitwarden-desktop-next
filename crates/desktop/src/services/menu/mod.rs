use std::sync::OnceLock;

use iced::futures::{SinkExt, Stream};
use muda::{Menu, MenuItem as MudaMenuItem, PredefinedMenuItem, Submenu};
use tokio::sync::broadcast;

// ---------------------------------------------------------------------------
// Shortcut
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    Char(char),
    F(u8),
}

#[derive(Clone, Copy)]
pub struct Shortcut {
    /// Ctrl on Win/Linux, Cmd on macOS.
    pub ctrl_cmd: bool,
    pub shift: bool,
    pub key: ShortcutKey,
}

const fn cmd(key: char) -> Shortcut {
    Shortcut {
        ctrl_cmd: true,
        shift: false,
        key: ShortcutKey::Char(key),
    }
}

const fn cmd_shift(key: char) -> Shortcut {
    Shortcut {
        ctrl_cmd: true,
        shift: true,
        key: ShortcutKey::Char(key),
    }
}

const fn fkey(n: u8) -> Shortcut {
    Shortcut {
        ctrl_cmd: false,
        shift: false,
        key: ShortcutKey::F(n),
    }
}

impl Shortcut {
    /// Platform-aware display text (e.g. "Ctrl+N" on Win, "Cmd+N" on Mac).
    pub fn display(&self) -> String {
        let mut s = String::new();
        if cfg!(target_os = "macos") {
            if self.ctrl_cmd {
                s.push_str("Cmd+");
            }
            if self.shift {
                s.push_str("Shift+");
            }
        } else {
            if self.ctrl_cmd {
                s.push_str("Ctrl+");
            }
            if self.shift {
                s.push_str("Shift+");
            }
        }
        match self.key {
            ShortcutKey::Char(c) if c.is_ascii_alphabetic() => s.push(c.to_ascii_uppercase()),
            ShortcutKey::Char(c) => s.push(c),
            ShortcutKey::F(n) => {
                s.push('F');
                s.push_str(&n.to_string());
            }
        }
        s
    }

    pub fn to_accelerator(self) -> Option<muda::accelerator::Accelerator> {
        self.display().parse().ok()
    }

    pub fn matches(&self, key: &iced::keyboard::Key, modifiers: iced::keyboard::Modifiers) -> bool {
        if modifiers.command() != self.ctrl_cmd {
            return false;
        }
        if modifiers.shift() != self.shift {
            return false;
        }

        match (&self.key, key) {
            (ShortcutKey::Char(c), iced::keyboard::Key::Character(s)) => s
                .chars()
                .next()
                .is_some_and(|pressed| pressed.eq_ignore_ascii_case(c)),
            (ShortcutKey::F(n), iced::keyboard::Key::Named(named)) => {
                use iced::keyboard::key::Named::*;
                let matched = match named {
                    F1 => 1,
                    F2 => 2,
                    F3 => 3,
                    F4 => 4,
                    F5 => 5,
                    F6 => 6,
                    F7 => 7,
                    F8 => 8,
                    F9 => 9,
                    F10 => 10,
                    F11 => 11,
                    F12 => 12,
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

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// When `true`, `label` is rendered verbatim instead of being looked up
    /// as a Fluent key. Use `L()` instead of `E()` for brand names and
    /// platform names ("Chrome", "iOS", …) that shouldn't be translated.
    pub literal: bool,
}

const SEP: MenuEntry = MenuEntry {
    label: "",
    shortcut: None,
    enabled: EnabledWhen::Always,
    action: None,
    children: &[],
    literal: false,
};

/// `label` is a Fluent message ID.
#[allow(non_snake_case)]
const fn E(label: &'static str) -> MenuEntry {
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
#[allow(non_snake_case)]
const fn L(label: &'static str) -> MenuEntry {
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
    const fn key(mut self, s: Shortcut) -> Self {
        self.shortcut = Some(s);
        self
    }
    const fn when(mut self, e: EnabledWhen) -> Self {
        self.enabled = e;
        self
    }
    const fn action(mut self, a: MenuAction) -> Self {
        self.action = Some(a);
        self
    }
    const fn sub(mut self, items: &'static [MenuEntry]) -> Self {
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

// ---------------------------------------------------------------------------
// Menu definitions
// ---------------------------------------------------------------------------

use EnabledWhen::*;
use MenuAction::*;

// Menu labels are Fluent message IDs resolved via `i18n::lookup()`. Brand /
// platform names are plain strings (no translation). Unlike `fl!()`, these
// keys aren't compile-time-checked — `tests/menu_labels.rs` validates them.
pub const MENUS: &[(&str, &[MenuEntry])] = &[
    (
        "menu-file",
        &[
            E("menu-file-new-login").key(cmd('n')).when(Unlocked),
            E("menu-file-new-item").when(Unlocked).sub(&[
                E("menu-file-new-item-login").key(cmd_shift('l')),
                E("menu-file-new-item-card").key(cmd_shift('c')),
                E("menu-file-new-item-identity").key(cmd_shift('i')),
                E("menu-file-new-item-secure-note").key(cmd_shift('s')),
                E("menu-file-new-item-ssh-key").key(cmd_shift('k')),
            ]),
            E("menu-file-new-folder").when(Unlocked).action(NewFolder),
            SEP,
            E("menu-file-sync-now").when(HasAccounts).action(SyncNow),
            E("menu-file-import").when(Unlocked).action(Import),
            E("menu-file-export").when(Unlocked).action(Export),
            SEP,
            E("menu-file-settings")
                .key(cmd(','))
                .when(Unlocked)
                .action(Settings),
            // Dynamically populated with account emails at runtime.
            E("menu-file-lock-vault").when(HasLockable).sub(&[]),
            E("menu-file-lock-all-vaults")
                .key(cmd('l'))
                .when(HasAccounts)
                .action(LockAllVaults),
            E("menu-file-log-out").when(HasAccounts).sub(&[]),
            SEP,
            E("menu-file-quit").action(Quit),
        ],
    ),
    (
        "menu-edit",
        &[
            // Undo/Redo/Cut/Copy/Paste/Select-all: text widgets handle these
            // via the keyboard already; menu wiring would need a focused-
            // widget dispatcher we haven't built. Disabled placeholders for
            // now — see docs/todo.md.
            E("menu-edit-undo").key(cmd('z')).when(Never),
            E("menu-edit-redo").key(cmd('y')).when(Never),
            SEP,
            E("menu-edit-cut").key(cmd('x')).when(Never),
            E("menu-edit-copy").key(cmd('c')).when(Never),
            E("menu-edit-paste").key(cmd('v')).when(Never),
            SEP,
            E("menu-edit-select-all").key(cmd('a')).when(Never),
            SEP,
            E("menu-edit-copy-username")
                .key(cmd('u'))
                .when(Unlocked)
                .action(CopyUsername),
            E("menu-edit-copy-password")
                .key(cmd('p'))
                .when(Unlocked)
                .action(CopyPassword),
            E("menu-edit-copy-totp")
                .key(cmd('t'))
                .when(Unlocked)
                .action(CopyTotp),
        ],
    ),
    (
        "menu-view",
        &[
            E("menu-view-search")
                .key(cmd('f'))
                .when(Unlocked)
                .action(SearchVault),
            SEP,
            E("menu-view-generator")
                .key(cmd('g'))
                .when(Unlocked)
                .action(Generator),
            E("menu-view-generator-history")
                .when(Unlocked)
                .action(GeneratorHistory),
            SEP,
            // Bound to '=' rather than '+' so users on US/EU layouts hit the
            // shortcut without holding shift — matches Firefox / Chrome.
            E("menu-view-zoom-in").key(cmd('=')).action(ZoomIn),
            E("menu-view-zoom-out").key(cmd('-')).action(ZoomOut),
            E("menu-view-reset-zoom").key(cmd('0')).action(ZoomReset),
            SEP,
            E("menu-view-toggle-fullscreen")
                .key(fkey(11))
                .action(ToggleFullScreen),
        ],
    ),
    (
        "menu-account",
        &[
            E("menu-account-premium")
                .when(Unlocked)
                .action(OpenWebVault(Some("#/settings/subscription/premium"))),
            E("menu-account-change-password")
                .when(Unlocked)
                .action(OpenWebVault(Some(
                    "#/settings/security/change-master-password",
                ))),
            E("menu-account-two-step")
                .when(Unlocked)
                .action(OpenWebVault(Some("#/settings/security/two-factor"))),
            E("menu-account-fingerprint")
                .when(Unlocked)
                .action(FingerprintPhrase),
            SEP,
            E("menu-account-delete")
                .when(Unlocked)
                .action(OpenWebVault(Some(
                    "#/settings/security/delete-account",
                ))),
        ],
    ),
    (
        "menu-window",
        &[
            E("menu-window-minimize").key(cmd('m')).action(Minimize),
            E("menu-window-hide-to-tray")
                .key(cmd_shift('m'))
                .action(HideToTray),
            E("menu-window-always-on-top")
                .key(cmd_shift('t'))
                .action(ToggleAlwaysOnTop),
            SEP,
            E("menu-window-close").key(cmd('w')).action(Close),
        ],
    ),
    (
        "menu-help",
        &[
            E("menu-help-feedback").action(OpenStaticUrl("https://bitwarden.com/help")),
            E("menu-help-bug").action(OpenStaticUrl("https://github.com/bitwarden/clients/issues")),
            E("menu-help-legal").sub(&[
                E("menu-help-legal-tos").action(OpenStaticUrl("https://bitwarden.com/terms/")),
                E("menu-help-legal-privacy")
                    .action(OpenStaticUrl("https://bitwarden.com/privacy/")),
            ]),
            SEP,
            E("menu-help-follow").sub(&[
                L("Blog").action(OpenStaticUrl("https://blog.bitwarden.com")),
                L("Twitter").action(OpenStaticUrl("https://twitter.com/bitwarden")),
                L("Facebook").action(OpenStaticUrl("https://www.facebook.com/bitwarden/")),
                L("GitHub").action(OpenStaticUrl("https://github.com/bitwarden")),
                L("Mastodon").action(OpenStaticUrl("https://fosstodon.org/@bitwarden")),
            ]),
            SEP,
            E("menu-help-web-vault").action(OpenWebVault(None)),
            SEP,
            E("menu-help-mobile-app").sub(&[
                L("iOS").action(OpenStaticUrl(
                    "https://itunes.apple.com/app/bitwarden-free-password-manager/id1137397744?mt=8",
                )),
                L("Android").action(OpenStaticUrl(
                    "https://play.google.com/store/apps/details?id=com.x8bit.bitwarden",
                )),
            ]),
            E("menu-help-browser-extension").sub(&[
                L("Chrome").action(OpenStaticUrl(
                    "https://chromewebstore.google.com/detail/bitwarden-free-password-m/nngceckbapebfimnlniiiahkandclblb",
                )),
                L("Firefox").action(OpenStaticUrl(
                    "https://addons.mozilla.org/firefox/addon/bitwarden-password-manager/",
                )),
                L("Opera").action(OpenStaticUrl(
                    "https://addons.opera.com/extensions/details/bitwarden-free-password-manager/",
                )),
                L("Edge").action(OpenStaticUrl(
                    "https://microsoftedge.microsoft.com/addons/detail/jbkfoedolllekgbhcbcoahefnbanhhlh",
                )),
                L("Safari").action(OpenStaticUrl("https://bitwarden.com/download/")),
            ]),
            SEP,
            E("menu-help-troubleshooting")
                .sub(&[E("menu-help-troubleshooting-gpu").action(ToggleHardwareAcceleration)]),
            SEP,
            E("menu-help-about").action(About),
        ],
    ),
];

// ---------------------------------------------------------------------------
// Event forwarding (push callback → broadcast → iced subscription)
// ---------------------------------------------------------------------------

/// Broadcast fan-out for muda events. Every `MenuEvent` (from the native app
/// menu and the tray context menu — muda shares one receiver for both) is
/// pushed into this channel.
static MUDA_EVENTS: OnceLock<broadcast::Receiver<muda::MenuEvent>> = OnceLock::new();

/// Register muda's global event handler. Call once at startup, before any
/// menu is built. Subsequent calls are ignored (both `OnceLock::set` and
/// muda's own `OnceCell`-backed `set_event_handler` silently no-op after
/// the first).
pub fn install_event_handler() {
    let (tx, rx) = broadcast::channel(16);
    let _ = MUDA_EVENTS.set(rx);
    muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
        let _ = tx.send(event);
    }));
}

/// Iced-compatible stream of [`muda::MenuEvent`] values, one per muda-managed
/// menu click. Use as a `fn` pointer with [`iced::Subscription::run`].
pub fn muda_event_stream() -> impl Stream<Item = muda::MenuEvent> {
    use iced::futures::channel::mpsc;
    iced::stream::channel(16, |mut out: mpsc::Sender<_>| async move {
        let mut rx = MUDA_EVENTS
            .get()
            .expect("menu::install_event_handler must run before subscribing")
            .resubscribe();
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    if out.send(ev).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(dropped = n, "muda event subscriber lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

// ---------------------------------------------------------------------------
// Shortcut → action lookup (for keyboard handling)
// ---------------------------------------------------------------------------

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
// Native menu
// ---------------------------------------------------------------------------

use std::collections::HashMap;

pub struct NativeMenuHandle {
    pub actions: HashMap<muda::MenuId, MenuAction>,
    pub items: Vec<(MudaMenuItem, EnabledWhen)>,
}

impl NativeMenuHandle {
    /// Resolve a muda `MenuId` to a `MenuAction` if it belongs to the native
    /// app menu. Returns `None` for unrelated ids (e.g. tray menu events).
    /// The caller drains `muda::MenuEvent::receiver()` — a global singleton
    /// shared with the tray menu — and dispatches by id.
    pub fn resolve(&self, id: &muda::MenuId) -> Option<MenuAction> {
        self.actions.get(id).copied()
    }

    pub fn sync_enabled(&self, state: &MenuState) {
        for (item, when) in &self.items {
            item.set_enabled(when.check(state));
        }
    }
}

/// Returns `None` if native menu is not enabled.
pub fn attach_menu(raw_id: u64) -> Option<NativeMenuHandle> {
    if !should_use_native_title_bar() {
        return None;
    }

    let mut actions = HashMap::new();
    let mut items = Vec::new();

    let menu = Menu::new();
    for (label_key, entries) in MENUS {
        let label = crate::services::i18n::lookup(label_key);
        let submenu = Submenu::new(format!("&{label}"), true);
        append_entries_to_submenu(&submenu, entries, &mut actions, &mut items);
        let _ = menu.append(&submenu);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = raw_id;
        menu.init_for_nsapp();
    }
    #[cfg(target_os = "windows")]
    {
        unsafe {
            let _ = menu.init_for_hwnd(raw_id as isize);
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // `should_use_native_title_bar()` returns false on Linux so the
        // early return above fires first; reaching here would mean someone
        // forced the native path on an unsupported platform.
        let _ = raw_id;
        let _ = menu;
        return None;
    }

    // The `Menu` and its `MenuItem`s must outlive the process — muda's
    // `init_for_{nsapp,hwnd}` hands ownership to the OS, which holds the
    // references until shutdown. Dropping the `Menu` would leave the OS
    // with a dangling pointer. Leaking is the idiomatic way to hand
    // ownership off to the platform menu system.
    std::mem::forget(menu);

    Some(NativeMenuHandle { actions, items })
}

pub fn should_use_custom_menu_bar() -> bool {
    cfg!(not(target_os = "macos")) || std::env::var("DEV_BOTH_MENUS").is_ok()
}

pub fn should_use_native_title_bar() -> bool {
    cfg!(target_os = "macos") || std::env::var("DEV_BOTH_MENUS").is_ok()
}

fn append_entries_to_submenu(
    submenu: &Submenu,
    entries: &[MenuEntry],
    actions: &mut HashMap<muda::MenuId, MenuAction>,
    items: &mut Vec<(MudaMenuItem, EnabledWhen)>,
) {
    let menu_items: Vec<Box<dyn muda::IsMenuItem>> = entries
        .iter()
        .map(|entry| -> Box<dyn muda::IsMenuItem> {
            if entry.is_separator() {
                Box::new(PredefinedMenuItem::separator())
            } else if !entry.children.is_empty() {
                let sub = Submenu::new(entry.display_label(), true);
                append_entries_to_submenu(&sub, entry.children, actions, items);
                Box::new(sub)
            } else {
                let accel = entry.shortcut.and_then(|s| s.to_accelerator());
                let item = MudaMenuItem::new(entry.display_label(), true, accel);
                if let Some(action) = entry.action {
                    actions.insert(item.id().clone(), action);
                }
                items.push((item.clone(), entry.enabled));
                Box::new(item)
            }
        })
        .collect();
    let refs: Vec<&dyn muda::IsMenuItem> = menu_items.iter().map(|b| b.as_ref()).collect();
    let _ = submenu.append_items(&refs);
}
