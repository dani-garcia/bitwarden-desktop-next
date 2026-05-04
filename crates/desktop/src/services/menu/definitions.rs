//! The static [`MENUS`] table — the single source of truth for both the
//! custom title-bar menu (Win/Linux) and the native muda menu (macOS).
//!
//! Also hosts [`find_shortcut_action`] which the keyboard-event handler uses
//! to dispatch shortcuts that aren't already routed through a focused widget.

use bitwarden_vault::CipherType;

use super::{
    EnabledWhen::*,
    MenuAction::*,
    entry::{E, L, MenuAction, MenuEntry, MenuState, SEP},
    shortcut::{cmd, cmd_shift, fkey},
};

// Menu labels are Fluent message IDs resolved via `i18n::lookup()`. Brand /
// platform names are plain strings (no translation). Unlike `fl!()`, these
// keys aren't compile-time-checked — `tests/menu_labels.rs` validates them.
pub const MENUS: &[(&str, &[MenuEntry])] = &[
    (
        "menu-file",
        &[
            E("menu-file-new-login")
                .key(cmd('n'))
                .when(Unlocked)
                .action(NewItem(CipherType::Login)),
            E("menu-file-new-item").when(Unlocked).sub(&[
                E("menu-file-new-item-login")
                    .key(cmd_shift('l'))
                    .action(NewItem(CipherType::Login)),
                E("menu-file-new-item-card")
                    .key(cmd_shift('c'))
                    .action(NewItem(CipherType::Card)),
                E("menu-file-new-item-identity")
                    .key(cmd_shift('i'))
                    .action(NewItem(CipherType::Identity)),
                E("menu-file-new-item-secure-note")
                    .key(cmd_shift('s'))
                    .action(NewItem(CipherType::SecureNote)),
                E("menu-file-new-item-ssh-key")
                    .key(cmd_shift('k'))
                    .action(NewItem(CipherType::SshKey)),
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
