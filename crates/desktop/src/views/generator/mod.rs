//! Generator modal — Password / Passphrase / Username tabs plus an in-memory
//! history panel.
//!
//! Opens via View → Generator (Ctrl/Cmd+G) and View → Generator history.
//! Any form-field change auto-regenerates: the view mutates local state,
//! emits [`GeneratorEvent::Generate`], and App dispatches the SDK call via
//! [`ClientManager::generate_password`] / `_passphrase` / `_username`.
//!
//! Every generated value is appended to the user's `password_history` on
//! `ClientManager`; the `Generated` result carries back the fresh history
//! snapshot so the view can rebind without a second round-trip.
//!
//! Closes via the X button, backdrop click, or Escape — same scaffolding as
//! the Settings modal ([`crate::views::settings`]).

mod handler;
mod history;
mod tabs;

use bitwarden_generators::{
    PassphraseGeneratorRequest, PasswordGeneratorRequest, UsernameGeneratorRequest,
};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding, Shadow,
    widget::{Space, button, column, container, row, scrollable, text},
};

use crate::{
    app::{Outcome, ViewTypes},
    components::{buttons, icons, modal, toast::Toast},
    fl,
    services::sdk::PasswordHistoryEntry,
    theme::{AppColors, AppTheme, RADIUS_LG, RADIUS_MD, RADIUS_PILL, RADIUS_SM},
};

// ── State ──────────────────────────────────────────────────────────────────

pub struct GeneratorView {
    pub open: bool,
    mode: Mode,
    active_tab: TabKind,
    password: PasswordForm,
    passphrase: PassphraseForm,
    username: UsernameForm,
    /// Latest generated value for the active tab. `None` until the first
    /// successful generation after open / tab switch.
    current: Option<String>,
    /// Cached snapshot of `ClientManager::password_history` for the active
    /// user. App refreshes this on modal open and after every successful
    /// `Generated` result.
    history: Vec<PasswordHistoryEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
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
    const ALL: &'static [Self] = &[Self::Password, Self::Passphrase, Self::Username];

    fn label(self) -> String {
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
    const ALL: &'static [Self] = &[Self::Word, Self::Subaddress, Self::Catchall];

    fn label(self) -> String {
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

// ── Messages + Events ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum GeneratorMessage {
    Close,
    SelectTab(TabKind),
    ShowHistory,
    BackToGenerator,
    ClearHistory,
    CopyCurrent,
    CopyHistoryEntry(usize),
    Regenerate,
    // Password
    SetLength(String),
    BumpLength(i32),
    ToggleLowercase(bool),
    ToggleUppercase(bool),
    ToggleNumbers(bool),
    ToggleSpecial(bool),
    SetMinNumber(String),
    BumpMinNumber(i32),
    SetMinSpecial(String),
    BumpMinSpecial(i32),
    ToggleAvoidAmbiguous(bool),
    // Passphrase
    SetNumWords(String),
    BumpNumWords(i32),
    SetWordSeparator(String),
    TogglePassphraseCapitalize(bool),
    TogglePassphraseIncludeNumber(bool),
    // Username
    SelectUsernameKind(UsernameKind),
    ToggleUsernameCapitalize(bool),
    ToggleUsernameIncludeNumber(bool),
    SetEmail(String),
    SetDomain(String),
    // Async round-trip from ClientManager
    Generated(Result<(String, Vec<PasswordHistoryEntry>), String>),
}

/// Events bubbled up to App. App is responsible for actually running the
/// SDK call (needs `Arc<ClientManager>` + active user) and for the
/// clipboard/toast side effects.
pub enum GeneratorEvent {
    /// Regenerate the output for the active tab. App spawns a Task that
    /// calls `ClientManager::generate_*` and pipes the result back through
    /// [`GeneratorMessage::Generated`].
    Generate(GenerateKind),
    /// User clicked the "Clear history" button inside the history panel.
    ClearHistory,
    /// Copy to clipboard + fire the "copied" toast.
    Copy(String),
    /// Generic toast request (currently only used on SDK error).
    Toast(Toast),
}

/// The concrete request payload for a regeneration. Built by the view from
/// its form state so App doesn't need to peek at private fields.
pub enum GenerateKind {
    Password(PasswordGeneratorRequest),
    Passphrase(PassphraseGeneratorRequest),
    Username(UsernameGeneratorRequest),
}

impl ViewTypes for GeneratorView {
    type Message = GeneratorMessage;
    type Event = GeneratorEvent;
}

// ── Lifecycle ──────────────────────────────────────────────────────────────

impl GeneratorView {
    pub fn new() -> Self {
        Self {
            open: false,
            mode: Mode::Generator,
            active_tab: TabKind::Password,
            password: PasswordForm::default(),
            passphrase: PassphraseForm::default(),
            username: UsernameForm::default(),
            current: None,
            history: Vec::new(),
        }
    }

    pub fn open_as_generator(&mut self) {
        self.open = true;
        self.mode = Mode::Generator;
        self.active_tab = TabKind::Password;
        self.current = None;
    }

    pub fn open_as_history(&mut self) {
        self.open = true;
        self.mode = Mode::History;
    }

    /// Replace the cached history snapshot. App calls this on modal open
    /// and after `ClearHistory` so the panel reflects storage.
    pub fn set_history(&mut self, history: Vec<PasswordHistoryEntry>) {
        self.history = history;
    }

    /// Build the regeneration request for the currently-active tab.
    /// Exposed to App so the menu-open path can dispatch an initial
    /// generation without going through the update loop.
    pub fn current_request(&self) -> GenerateKind {
        match self.active_tab {
            TabKind::Password => GenerateKind::Password(self.password.to_request()),
            TabKind::Passphrase => GenerateKind::Passphrase(self.passphrase.to_request()),
            TabKind::Username => GenerateKind::Username(self.username.to_request()),
        }
    }

    pub fn update(
        &mut self,
        msg: GeneratorMessage,
        _ctx: crate::app::UpdateCtx<'_>,
    ) -> Outcome<Self> {
        match msg {
            GeneratorMessage::Close => {
                self.open = false;
                Outcome::None
            }
            GeneratorMessage::SelectTab(tab) => {
                if self.active_tab == tab {
                    return Outcome::None;
                }
                self.active_tab = tab;
                self.current = None;
                self.regenerate_event()
            }
            GeneratorMessage::ShowHistory => {
                self.mode = Mode::History;
                Outcome::None
            }
            GeneratorMessage::BackToGenerator => {
                self.mode = Mode::Generator;
                // Seed the value card if the modal was opened directly into
                // history mode and the active tab has never generated yet.
                if self.current.is_none() {
                    return self.regenerate_event();
                }
                Outcome::None
            }
            GeneratorMessage::ClearHistory => {
                self.history.clear();
                Outcome::event(GeneratorEvent::ClearHistory)
            }
            GeneratorMessage::CopyCurrent => match self.current.clone() {
                Some(v) if !v.is_empty() => Outcome::event(GeneratorEvent::Copy(v)),
                _ => Outcome::None,
            },
            GeneratorMessage::CopyHistoryEntry(idx) => {
                // History is rendered reverse-chronologically; the idx the
                // row passes is into that reversed view, so translate back
                // to the storage order (oldest-first).
                let len = self.history.len();
                if idx >= len {
                    return Outcome::None;
                }
                let storage_idx = len - 1 - idx;
                let value = self.history[storage_idx].value.clone();
                Outcome::event(GeneratorEvent::Copy(value))
            }
            GeneratorMessage::Regenerate => self.regenerate_event(),
            // Password tab
            GeneratorMessage::SetLength(raw) => {
                if accept_digits(&raw) {
                    self.password.length = raw;
                    return self.regenerate_event();
                }
                Outcome::None
            }
            GeneratorMessage::BumpLength(delta) => {
                self.password.length = bump_clamped(&self.password.length, delta, 14, 5, 128);
                self.regenerate_event()
            }
            GeneratorMessage::ToggleLowercase(v) => {
                self.password.lowercase = v;
                self.regenerate_event()
            }
            GeneratorMessage::ToggleUppercase(v) => {
                self.password.uppercase = v;
                self.regenerate_event()
            }
            GeneratorMessage::ToggleNumbers(v) => {
                self.password.numbers = v;
                self.regenerate_event()
            }
            GeneratorMessage::ToggleSpecial(v) => {
                self.password.special = v;
                self.regenerate_event()
            }
            GeneratorMessage::SetMinNumber(raw) => {
                if accept_digits(&raw) {
                    self.password.min_number = raw;
                    return self.regenerate_event();
                }
                Outcome::None
            }
            GeneratorMessage::BumpMinNumber(delta) => {
                self.password.min_number =
                    bump_clamped(&self.password.min_number, delta, 1, 0, 9);
                self.regenerate_event()
            }
            GeneratorMessage::SetMinSpecial(raw) => {
                if accept_digits(&raw) {
                    self.password.min_special = raw;
                    return self.regenerate_event();
                }
                Outcome::None
            }
            GeneratorMessage::BumpMinSpecial(delta) => {
                self.password.min_special =
                    bump_clamped(&self.password.min_special, delta, 1, 0, 9);
                self.regenerate_event()
            }
            GeneratorMessage::ToggleAvoidAmbiguous(v) => {
                self.password.avoid_ambiguous = v;
                self.regenerate_event()
            }
            // Passphrase tab
            GeneratorMessage::SetNumWords(raw) => {
                if accept_digits(&raw) {
                    self.passphrase.num_words = raw;
                    return self.regenerate_event();
                }
                Outcome::None
            }
            GeneratorMessage::BumpNumWords(delta) => {
                self.passphrase.num_words =
                    bump_clamped(&self.passphrase.num_words, delta, 6, 3, 20);
                self.regenerate_event()
            }
            GeneratorMessage::SetWordSeparator(s) => {
                // Cap at 1 character; the SDK's `word_separator` is a
                // `String` but the screenshot shows a single-char field.
                self.passphrase.word_separator = s.chars().take(1).collect();
                self.regenerate_event()
            }
            GeneratorMessage::TogglePassphraseCapitalize(v) => {
                self.passphrase.capitalize = v;
                self.regenerate_event()
            }
            GeneratorMessage::TogglePassphraseIncludeNumber(v) => {
                self.passphrase.include_number = v;
                self.regenerate_event()
            }
            // Username tab
            GeneratorMessage::SelectUsernameKind(k) => {
                self.username.kind = k;
                self.regenerate_event()
            }
            GeneratorMessage::ToggleUsernameCapitalize(v) => {
                self.username.capitalize = v;
                self.regenerate_event()
            }
            GeneratorMessage::ToggleUsernameIncludeNumber(v) => {
                self.username.include_number = v;
                self.regenerate_event()
            }
            GeneratorMessage::SetEmail(s) => {
                self.username.email = s;
                self.regenerate_event()
            }
            GeneratorMessage::SetDomain(s) => {
                self.username.domain = s;
                self.regenerate_event()
            }
            GeneratorMessage::Generated(Ok((value, history))) => {
                self.current = Some(value);
                self.history = history;
                Outcome::None
            }
            GeneratorMessage::Generated(Err(err)) => {
                tracing::warn!(%err, "generator request failed");
                Outcome::event(GeneratorEvent::Toast(Toast::warning(
                    err,
                    Some(&fl!("generator-toast-failed")),
                )))
            }
        }
    }

    fn regenerate_event(&self) -> Outcome<Self> {
        Outcome::event(GeneratorEvent::Generate(self.current_request()))
    }

    // ── view ──────────────────────────────────────────────────────────────

    /// Returns `None` when the modal is closed so App's view composer can
    /// take a cheap exclusive branch (see CLAUDE.md → "Stack doesn't cull").
    pub fn modal_view<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Option<Element<'a, GeneratorMessage, AppTheme>> {
        if !self.open {
            return None;
        }

        let body = match self.mode {
            Mode::Generator => self.generator_body(colors),
            Mode::History => history::view(&self.history, colors),
        };

        // Header: title on left, X close on right. Title varies with mode.
        let title_label = match self.mode {
            Mode::Generator => fl!("generator-title"),
            Mode::History => fl!("generator-history-title"),
        };
        let header = row![
            text(title_label)
                .size(20)
                .font(crate::APP_FONT_BOLD)
                .color(colors.text_primary),
            Space::new().width(Fill),
            buttons::ghost_icon(
                icons::X_LG.render(16.0, colors.text_primary),
                colors.item_hover,
            )
            .padding([6, 6])
            .on_press(GeneratorMessage::Close),
        ]
        .align_y(Alignment::Center);

        // Right-pad the body inside the scrollable so fields don't kiss
        // the rail. Match the cipher list's muted scroller styling so the
        // bar isn't a stark white sliver against the dark dialog.
        let scrolled = scrollable(
            container(body)
                .padding(Padding {
                    top: 0.0,
                    right: 12.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .width(Fill),
        )
        .height(Fill)
        .style(|theme: &AppTheme, _status| scrollable::Style {
            container: container::Style::default(),
            vertical_rail: scrollable::Rail {
                background: None,
                border: Border::default(),
                scroller: scrollable::Scroller {
                    background: Background::Color(theme.colors.item_hover),
                    border: Border::default().rounded(4),
                },
            },
            horizontal_rail: scrollable::Rail {
                background: None,
                border: Border::default(),
                scroller: scrollable::Scroller {
                    background: Background::Color(theme.colors.item_hover),
                    border: Border::default().rounded(4),
                },
            },
            gap: None,
            auto_scroll: scrollable::AutoScroll {
                background: Background::Color(Color::TRANSPARENT),
                border: Border::default(),
                shadow: Shadow::default(),
                icon: Color::TRANSPARENT,
            },
        });

        let content = column![header, Space::new().height(16), scrolled,]
            .width(Fill)
            .height(Fill);

        let dialog = container(
            container(content)
                .padding(Padding::from([20, 24]))
                .width(Fill)
                .height(Fill),
        )
        .width(Length::Fixed(680.0))
        .height(Length::Fixed(620.0))
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(Border::default().rounded(RADIUS_LG))
        });

        Some(modal::view(dialog.into(), GeneratorMessage::Close))
    }

    fn generator_body<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Element<'a, GeneratorMessage, AppTheme> {
        let tabs = tab_row(self.active_tab, colors);
        let value_card = value_card(self.current.as_deref().unwrap_or(""), colors);
        let options_heading = section_heading(fl!("generator-options"), colors);
        let options: Element<'a, GeneratorMessage, AppTheme> = match self.active_tab {
            TabKind::Password => tabs::password::view(&self.password, colors),
            TabKind::Passphrase => tabs::passphrase::view(&self.passphrase, colors),
            TabKind::Username => tabs::username::view(&self.username, colors),
        };

        let history_row = history_entry_row(colors);

        column![
            tabs,
            Space::new().height(12),
            value_card,
            Space::new().height(14),
            options_heading,
            Space::new().height(6),
            options,
            Space::new().height(14),
            history_row,
        ]
        .width(Fill)
        .into()
    }
}

impl Default for GeneratorView {
    fn default() -> Self {
        Self::new()
    }
}

// ── Form → SDK request adapters ────────────────────────────────────────────

impl PasswordForm {
    fn to_request(&self) -> PasswordGeneratorRequest {
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
    fn to_request(&self) -> PassphraseGeneratorRequest {
        PassphraseGeneratorRequest {
            num_words: parse_u8(&self.num_words, 6, 3, 20),
            word_separator: self.word_separator.clone(),
            capitalize: self.capitalize,
            include_number: self.include_number,
        }
    }
}

impl UsernameForm {
    fn to_request(&self) -> UsernameGeneratorRequest {
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
            .expect("username subaddress request JSON matches SDK schema"),
            UsernameKind::Catchall => serde_json::from_value(serde_json::json!({
                "catchall": {
                    "type": "random",
                    "domain": self.domain,
                }
            }))
            .expect("username catchall request JSON matches SDK schema"),
        }
    }
}

// ── Shared render helpers ──────────────────────────────────────────────────

pub(super) fn section_heading<'a>(
    label: impl Into<String>,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    text(label.into())
        .size(16)
        .font(crate::APP_FONT_BOLD)
        .color(colors.text_primary)
        .into()
}

/// Segmented tab bar (three buttons in a rounded pill). Active tab uses
/// the accent color; inactive tabs are transparent with hover.
fn tab_row<'a>(
    active: TabKind,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let mut r = row![].spacing(2);
    for &k in TabKind::ALL {
        r = r.push(tab_button(k, k == active, colors));
    }

    container(r)
        .padding(Padding::from([2, 2]))
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(RADIUS_PILL))
        })
        .into()
}

fn tab_button<'a>(
    kind: TabKind,
    active: bool,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let label_color = if active {
        colors.text_primary
    } else {
        colors.text_secondary
    };
    let active_bg = colors.accent;
    let hover_bg = colors.item_hover;

    let label = container(
        text(kind.label())
            .size(13)
            .color(label_color)
            .align_x(Alignment::Center),
    )
    .width(Fill)
    .align_x(Alignment::Center)
    .padding([4, 8]);

    buttons::ghost(label, active, active_bg, hover_bg, RADIUS_PILL)
        .width(Fill)
        .on_press(GeneratorMessage::SelectTab(kind))
        .into()
}

/// Big value display with refresh + copy buttons on the right. Monospace
/// font for the value so long passwords align predictably.
fn value_card<'a>(
    value: &'a str,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let value_text = text(value.to_string())
        .size(16)
        .font(iced::Font::MONOSPACE)
        .color(colors.text_primary);

    let refresh = buttons::ghost_icon(
        icons::ARROW_REPEAT.render(18.0, colors.text_primary),
        colors.item_hover,
    )
    .padding([6, 6])
    .on_press(GeneratorMessage::Regenerate);

    let copy = buttons::ghost_icon(
        icons::BWI_COPY.render(18.0, colors.text_primary),
        colors.item_hover,
    )
    .padding([6, 6])
    .on_press(GeneratorMessage::CopyCurrent);

    container(
        row![value_text, Space::new().width(Fill), refresh, copy]
            .align_y(Alignment::Center)
            .spacing(4),
    )
    .padding(Padding::from([12, 16]))
    .width(Fill)
    .style(|theme: &AppTheme| {
        container::Style::default()
            .background(theme.colors.card_bg)
            .border(Border::default().rounded(RADIUS_MD))
    })
    .into()
}

/// "Generator history >" disclosure row at the bottom of the generator
/// body. Tapping anywhere on the row flips the modal into history mode.
fn history_entry_row<'a>(
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let row_content = row![
        text(fl!("generator-history-open"))
            .size(14)
            .color(colors.accent)
            .font(crate::APP_FONT_BOLD),
        Space::new().width(Fill),
        icons::CHEVRON_RIGHT.render(14.0, colors.accent),
    ]
    .align_y(Alignment::Center);

    let padded = container(row_content)
        .padding(Padding::from([12, 16]))
        .width(Fill);

    button(padded)
        .width(Fill)
        .padding(0)
        .style(move |_theme: &AppTheme, status| {
            let bg = match status {
                button::Status::Hovered => Background::Color(colors.item_hover),
                _ => Background::Color(Color::TRANSPARENT),
            };
            button::Style {
                background: Some(bg),
                text_color: colors.text_primary,
                border: Border::default().rounded(RADIUS_SM),
                shadow: iced::Shadow::default(),
                snap: false,
            }
        })
        .on_press(GeneratorMessage::ShowHistory)
        .into()
}

/// Parse a user-typed number string to a `u8`, clamping to `[min, max]`.
/// Empty / invalid input falls back to `default`. Used when building the
/// SDK request so garbage never reaches the generator.
fn parse_u8(raw: &str, default: u8, min: u8, max: u8) -> u8 {
    raw.trim()
        .parse::<u32>()
        .map(|n| n.min(max as u32).max(min as u32) as u8)
        .unwrap_or(default)
}

/// Reject keystrokes that aren't digits — keeps `length`, `min_number`,
/// etc. fields from accepting `"abc"` while still allowing the user to
/// clear the field temporarily ("" parses to default).
fn accept_digits(raw: &str) -> bool {
    raw.is_empty() || raw.chars().all(|c| c.is_ascii_digit())
}

/// Apply a stepper delta (`+1` / `-1`) to a numeric field's raw string,
/// clamping the result. Empty / invalid current value uses `default`.
fn bump_clamped(raw: &str, delta: i32, default: u8, min: u8, max: u8) -> String {
    let current = raw.trim().parse::<i32>().unwrap_or(default as i32);
    let next = (current + delta).clamp(min as i32, max as i32);
    next.to_string()
}
