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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::menu::{EnabledWhen, shortcut::ShortcutKey};
    use iced::keyboard::{Key, Modifiers, key::Named};

    fn key_char(c: &str) -> Key {
        Key::Character(c.into())
    }

    fn unlocked() -> MenuState {
        MenuState {
            is_locked: false,
            has_accounts: true,
            has_lockable_accounts: true,
        }
    }

    fn locked_no_accounts() -> MenuState {
        MenuState {
            is_locked: true,
            has_accounts: false,
            has_lockable_accounts: false,
        }
    }

    fn locked_with_accounts() -> MenuState {
        MenuState {
            is_locked: true,
            has_accounts: true,
            has_lockable_accounts: false,
        }
    }

    // The platform-aware modifier — `Modifiers::COMMAND` resolves to LOGO on
    // macOS and CTRL elsewhere, matching `Shortcut::matches`.
    fn cmd_only() -> Modifiers {
        Modifiers::COMMAND
    }
    fn cmd_shift_mods() -> Modifiers {
        Modifiers::COMMAND | Modifiers::SHIFT
    }

    // ── find_shortcut_action: enable-gates ────────────────────────────────

    #[test]
    fn cmd_n_blocked_when_locked() {
        // File → New login is `Unlocked`-gated.
        assert_eq!(
            find_shortcut_action(&key_char("n"), cmd_only(), &locked_no_accounts()),
            None
        );
    }

    #[test]
    fn cmd_n_routes_to_new_login_when_unlocked() {
        let action = find_shortcut_action(&key_char("n"), cmd_only(), &unlocked());
        assert_eq!(
            action,
            Some(MenuAction::NewItem(bitwarden_vault::CipherType::Login))
        );
    }

    #[test]
    fn cmd_l_requires_has_accounts() {
        // Lock all vaults requires HasAccounts even when the vault is locked.
        assert_eq!(
            find_shortcut_action(&key_char("l"), cmd_only(), &locked_no_accounts()),
            None
        );
        assert_eq!(
            find_shortcut_action(&key_char("l"), cmd_only(), &locked_with_accounts()),
            Some(MenuAction::LockAllVaults)
        );
    }

    #[test]
    fn never_gated_entries_never_fire() {
        // Edit → Undo is `EnabledWhen::Never`. The shortcut must always
        // return `None` so it falls through to focused text widgets.
        for state in [unlocked(), locked_no_accounts(), locked_with_accounts()] {
            assert_eq!(
                find_shortcut_action(&key_char("z"), cmd_only(), &state),
                None,
                "cmd+Z fired in state {state:?}",
            );
        }
    }

    #[test]
    fn shortcut_in_submenu_resolves() {
        // File → New item → Secure note (cmd+shift+S) lives inside a sub.
        let action = find_shortcut_action(&key_char("s"), cmd_shift_mods(), &unlocked());
        assert_eq!(
            action,
            Some(MenuAction::NewItem(bitwarden_vault::CipherType::SecureNote))
        );
    }

    #[test]
    fn fkey_shortcut_with_no_modifiers_resolves() {
        // F11 → ToggleFullScreen, always-enabled.
        let action = find_shortcut_action(&Key::Named(Named::F11), Modifiers::empty(), &unlocked());
        assert_eq!(action, Some(MenuAction::ToggleFullScreen));
    }

    #[test]
    fn unbound_combination_returns_none() {
        // No menu binds cmd+shift+`q`.
        assert_eq!(
            find_shortcut_action(&key_char("q"), cmd_shift_mods(), &unlocked()),
            None
        );
    }

    #[test]
    fn shortcut_requires_correct_modifiers() {
        // cmd+`n` is bound; `n` alone (no modifiers) is not.
        assert_eq!(
            find_shortcut_action(&key_char("n"), Modifiers::empty(), &unlocked()),
            None
        );
        // Adding shift shouldn't match either.
        assert_eq!(
            find_shortcut_action(&key_char("n"), cmd_shift_mods(), &unlocked()),
            None
        );
    }

    // ── MENUS table-wide invariants ───────────────────────────────────────

    /// Recursively collect every `(modifiers, key)` triple bound by an entry
    /// that *can* fire (anything but `Never`).
    fn collect_active_shortcuts(
        entries: &[MenuEntry],
        out: &mut Vec<((bool, bool, ShortcutKey), &'static str)>,
    ) {
        for entry in entries {
            if let Some(s) = entry.shortcut
                && !matches!(entry.enabled, EnabledWhen::Never)
            {
                out.push(((s.ctrl_cmd, s.shift, s.key), entry.label));
            }
            collect_active_shortcuts(entry.children, out);
        }
    }

    #[test]
    fn active_shortcuts_have_no_collisions() {
        // A duplicate `(ctrl_cmd, shift, key)` between two non-`Never` entries
        // means `find_shortcut_action` returns whichever appears first in the
        // walk — the second one is silently unreachable.
        let mut all = Vec::new();
        for (_label, entries) in MENUS {
            collect_active_shortcuts(entries, &mut all);
        }
        for (i, (a, label_a)) in all.iter().enumerate() {
            for (b, label_b) in all.iter().skip(i + 1) {
                // `assert!` rather than `assert_ne!` to avoid requiring
                // `Debug` on `ShortcutKey` purely for test diagnostics.
                assert!(
                    a != b,
                    "shortcut collision between {label_a:?} and {label_b:?}"
                );
            }
        }
    }

    #[test]
    fn active_shortcuts_round_trip_through_muda() {
        // Each shortcut display string must be parseable as a
        // `muda::accelerator::Accelerator` — otherwise the native macOS menu
        // silently drops it. Cheap to assert here because our `display()`
        // and muda's parser are the contract we own.
        let mut all = Vec::new();
        for (_label, entries) in MENUS {
            collect_active_shortcuts(entries, &mut all);
        }
        for ((ctrl_cmd, shift, key), label) in all {
            let s = super::super::shortcut::Shortcut {
                ctrl_cmd,
                shift,
                key,
            };
            assert!(
                s.to_accelerator().is_some(),
                "shortcut for {label:?} ({}) didn't parse as a muda accelerator",
                s.display()
            );
        }
    }
}
