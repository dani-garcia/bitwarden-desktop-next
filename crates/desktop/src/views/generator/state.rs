//! Generator form state: mode/tab enums, per-tab form structs, SDK request
//! adapters, and number-input parse helpers.

use bitwarden_generators::{
    PassphraseGeneratorRequest, PasswordGeneratorRequest, UsernameGeneratorRequest,
};

use crate::fl;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Generator,
    History,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Password,
    Passphrase,
    Username,
}

impl TabKind {
    pub(super) const ALL: &'static [Self] = &[Self::Password, Self::Passphrase, Self::Username];

    pub(super) fn label(self) -> String {
        match self {
            Self::Password => fl!("generator-tab-password"),
            Self::Passphrase => fl!("generator-tab-passphrase"),
            Self::Username => fl!("generator-tab-username"),
        }
    }
}

/// Numeric fields are stored as `String` so the iced `text_input` can borrow
/// them directly and the user can type freely (including intermediate
/// invalid states like empty). `to_request` parses + clamps with a fallback
/// default, so the SDK always sees a valid value.
pub(super) struct PasswordForm {
    pub(super) length: String,
    pub(super) lowercase: bool,
    pub(super) uppercase: bool,
    pub(super) numbers: bool,
    pub(super) special: bool,
    pub(super) min_number: String,
    pub(super) min_special: String,
    pub(super) avoid_ambiguous: bool,
}

impl Default for PasswordForm {
    fn default() -> Self {
        Self {
            length: "14".to_string(),
            lowercase: true,
            uppercase: true,
            numbers: true,
            special: true,
            min_number: "1".to_string(),
            min_special: "1".to_string(),
            avoid_ambiguous: false,
        }
    }
}

pub(super) struct PassphraseForm {
    pub(super) num_words: String,
    pub(super) word_separator: String,
    pub(super) capitalize: bool,
    pub(super) include_number: bool,
}

impl Default for PassphraseForm {
    fn default() -> Self {
        Self {
            num_words: "6".to_string(),
            word_separator: "-".to_string(),
            capitalize: false,
            include_number: false,
        }
    }
}

pub(super) struct UsernameForm {
    pub(super) kind: UsernameKind,
    // Word
    pub(super) capitalize: bool,
    pub(super) include_number: bool,
    // Subaddress
    pub(super) email: String,
    // Catchall
    pub(super) domain: String,
}

impl Default for UsernameForm {
    fn default() -> Self {
        Self {
            kind: UsernameKind::Word,
            capitalize: false,
            include_number: false,
            email: String::new(),
            domain: String::new(),
        }
    }
}

/// Username strategies the UI supports. `Forwarded` is intentionally
/// excluded — it requires configured third-party API tokens (SimpleLogin,
/// DuckDuckGo, etc), which is out of scope for this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsernameKind {
    Word,
    Subaddress,
    Catchall,
}

impl UsernameKind {
    pub(super) const ALL: &'static [Self] = &[Self::Word, Self::Subaddress, Self::Catchall];

    pub(super) fn label(self) -> String {
        match self {
            Self::Word => fl!("generator-username-kind-word"),
            Self::Subaddress => fl!("generator-username-kind-subaddress"),
            Self::Catchall => fl!("generator-username-kind-catchall"),
        }
    }
}

impl std::fmt::Display for UsernameKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label())
    }
}

// ── Form → SDK request adapters ────────────────────────────────────────────

impl PasswordForm {
    pub(super) fn to_request(&self) -> PasswordGeneratorRequest {
        let length = parse_u8(&self.length, 14, 5, 128);
        let min_number = parse_u8(&self.min_number, 1, 0, 9);
        let min_special = parse_u8(&self.min_special, 1, 0, 9);
        PasswordGeneratorRequest {
            lowercase: self.lowercase,
            uppercase: self.uppercase,
            numbers: self.numbers,
            special: self.special,
            length,
            avoid_ambiguous: self.avoid_ambiguous,
            min_lowercase: None,
            min_uppercase: None,
            // Only supply minimums for the charsets the user actually
            // included — otherwise the SDK rejects the request with
            // `NoCharacterSetEnabled` / similar when it has to satisfy
            // a minimum from an unchecked group.
            min_number: self.numbers.then_some(min_number),
            min_special: self.special.then_some(min_special),
        }
    }
}

impl PassphraseForm {
    pub(super) fn to_request(&self) -> PassphraseGeneratorRequest {
        PassphraseGeneratorRequest {
            num_words: parse_u8(&self.num_words, 6, 3, 20),
            word_separator: self.word_separator.clone(),
            capitalize: self.capitalize,
            include_number: self.include_number,
        }
    }
}

impl UsernameForm {
    pub(super) fn to_request(&self) -> UsernameGeneratorRequest {
        match self.kind {
            UsernameKind::Word => UsernameGeneratorRequest::Word {
                capitalize: self.capitalize,
                include_number: self.include_number,
            },
            // `AppendType` isn't re-exported by `bitwarden-generators`, so
            // we can't name its variants directly. Deserialize from a JSON
            // shape instead — the SDK derives `Deserialize` with
            // `camelCase` + externally-tagged enums, matching the keys
            // below. `"random"` is the `AppendType::Random` unit variant.
            UsernameKind::Subaddress => serde_json::from_value(serde_json::json!({
                "subaddress": {
                    "type": "random",
                    "email": self.email,
                }
            }))
            .unwrap_or_else(|err| {
                tracing::error!(%err, "subaddress request schema mismatch; falling back to Word");
                UsernameGeneratorRequest::Word {
                    capitalize: false,
                    include_number: false,
                }
            }),
            UsernameKind::Catchall => serde_json::from_value(serde_json::json!({
                "catchall": {
                    "type": "random",
                    "domain": self.domain,
                }
            }))
            .unwrap_or_else(|err| {
                tracing::error!(%err, "catchall request schema mismatch; falling back to Word");
                UsernameGeneratorRequest::Word {
                    capitalize: false,
                    include_number: false,
                }
            }),
        }
    }
}

// ── Number-input helpers ──────────────────────────────────────────────────

/// Parse a user-typed number string to a `u8`, clamping to `[min, max]`.
/// Empty / invalid input falls back to `default`. Used when building the
/// SDK request so garbage never reaches the generator.
pub(super) fn parse_u8(raw: &str, default: u8, min: u8, max: u8) -> u8 {
    raw.trim()
        .parse::<u32>()
        .map(|n| n.min(max as u32).max(min as u32) as u8)
        .unwrap_or(default)
}

/// Reject keystrokes that aren't digits — keeps `length`, `min_number`,
/// etc. fields from accepting `"abc"` while still allowing the user to
/// clear the field temporarily ("" parses to default).
pub(super) fn accept_digits(raw: &str) -> bool {
    raw.is_empty() || raw.chars().all(|c| c.is_ascii_digit())
}

/// Apply a stepper delta (`+1` / `-1`) to a numeric field's raw string,
/// clamping the result. Empty / invalid current value uses `default`.
pub(super) fn bump_clamped(raw: &str, delta: i32, default: u8, min: u8, max: u8) -> String {
    let current = raw.trim().parse::<i32>().unwrap_or(default as i32);
    let next = (current + delta).clamp(min as i32, max as i32);
    next.to_string()
}
