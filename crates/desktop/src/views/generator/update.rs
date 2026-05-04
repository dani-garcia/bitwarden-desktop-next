//! `GeneratorView::update` — message dispatch for the generator modal.

use std::time::{Duration, Instant};

use crate::{
    app::{Outcome, UpdateCtx},
    components::toast::Toast,
    fl,
    services::animation,
};

use super::{
    GeneratorView, TAB_ANIM_MS,
    message::{GeneratorEvent, GeneratorMessage},
    state::{Mode, TabKind, accept_digits, bump_clamped},
};

impl GeneratorView {
    pub fn update(&mut self, msg: GeneratorMessage, ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            GeneratorMessage::Close => {
                self.fade.close();
                Outcome::None
            }
            GeneratorMessage::SelectTab(tab) => {
                if self.active_tab == tab {
                    return Outcome::None;
                }
                self.active_tab = tab;
                let new_index = TabKind::ALL.iter().position(|t| *t == tab).unwrap_or(0) as f32;
                self.tab_anim.transition(new_index, Instant::now());
                animation::extend(Duration::from_millis(TAB_ANIM_MS as u64));
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
                self.password.min_number = bump_clamped(&self.password.min_number, delta, 1, 0, 9);
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
            GeneratorMessage::Generated(Ok(value)) => {
                self.current = Some(value.clone());
                if let Some(uid) = ctx.active_user.copied() {
                    self.history = ctx.client_manager.push_history(uid, value);
                }
                Outcome::None
            }
            GeneratorMessage::Generated(Err(err)) => {
                tracing::warn!(%err, "generator request failed");
                Outcome::toast(Toast::warning(err, Some(&fl!("generator-toast-failed"))))
            }
        }
    }

    pub(super) fn regenerate_event(&self) -> Outcome<Self> {
        Outcome::event(GeneratorEvent::Generate(self.current_request()))
    }
}
