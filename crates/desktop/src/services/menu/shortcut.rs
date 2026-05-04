//! Keyboard shortcut representation. Used by both the custom title-bar menu
//! (which renders the shortcut display text alongside the label) and the
//! native muda menu (which converts to `muda::accelerator::Accelerator`).

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

pub(super) const fn cmd(key: char) -> Shortcut {
    Shortcut {
        ctrl_cmd: true,
        shift: false,
        key: ShortcutKey::Char(key),
    }
}

pub(super) const fn cmd_shift(key: char) -> Shortcut {
    Shortcut {
        ctrl_cmd: true,
        shift: true,
        key: ShortcutKey::Char(key),
    }
}

pub(super) const fn fkey(n: u8) -> Shortcut {
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
                matched == *n
            }
            _ => false,
        }
    }
}
