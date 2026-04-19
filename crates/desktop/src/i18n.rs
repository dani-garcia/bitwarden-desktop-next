//! Localization via [`i18n-embed`] + Fluent.
//!
//! Translation assets live at the workspace root under
//! `assets/i18n/{lang}/{crate}.ftl` alongside the other binary resources
//! (fonts, SVGs, icons). They are embedded into the binary at compile time
//! via [`rust_embed`] so the packaged binary carries all languages without
//! extra files on disk.
//!
//! ## Adding a string
//!
//! 1. Add a key + English value to `assets/i18n/en/bitwarden_desktop_next.ftl`.
//! 2. Use [`fl!`] at the call site: `fl!("my-key")` or `fl!("my-key", name = value)`.
//!
//! The [`fl!`] macro (defined at the crate root) wraps [`i18n_embed_fl::fl!`] so
//! the loader argument is implicit. Unknown keys or wrong argument names are a
//! compile error thanks to `i18n-embed-fl`'s static check against the `.ftl`
//! files.
//!
//! ## Runtime locale
//!
//! [`init`] is called once from `main` before the iced daemon starts. It reads
//! the OS locale via `DesktopLanguageRequester` and selects the best match from
//! the embedded languages, falling back to English when none match.
//!
//! To change language at runtime, call [`set_language`] from a message handler.
//! iced will re-render on the next frame and every `fl!()` call will return the
//! new translation — no restart required.

use std::sync::LazyLock;

use i18n_embed::{
    DesktopLanguageRequester, LanguageLoader,
    fluent::{FluentLanguageLoader, fluent_language_loader},
    unic_langid::LanguageIdentifier,
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../assets/i18n"]
pub(crate) struct Localizations;

#[doc(hidden)]
pub static LANGUAGE_LOADER: LazyLock<FluentLanguageLoader> = LazyLock::new(|| {
    let loader: FluentLanguageLoader = fluent_language_loader!();
    loader
        .load_fallback_language(&Localizations)
        .expect("fallback language (en) must load");
    loader
});

/// Select the OS-preferred language from the embedded set, falling back to the
/// loader's fallback language when no match exists. Call once at app startup.
pub fn init() {
    let requested = DesktopLanguageRequester::requested_languages();
    if let Err(e) = i18n_embed::select(&*LANGUAGE_LOADER, &Localizations, &requested) {
        tracing::warn!(%e, "i18n language selection failed; falling back to default");
    }
}

/// Switch to a specific language. Call from a message handler; iced re-renders
/// on the next frame and every `fl!()` call picks up the new translation.
///
/// Falls back to English (with a warning log) if the requested language has no
/// matching `.ftl` file under `assets/i18n/`.
#[expect(dead_code)] // Wired up once the settings view with a language picker lands.
pub fn set_language(lang: LanguageIdentifier) {
    if let Err(e) = i18n_embed::select(
        &*LANGUAGE_LOADER,
        &Localizations,
        std::slice::from_ref(&lang),
    ) {
        tracing::warn!(%e, %lang, "set_language failed; current language unchanged");
    }
}

/// Languages with at least one `.ftl` file under `assets/i18n/`. Useful for
/// populating a language picker.
#[expect(dead_code)] // Wired up once the settings view with a language picker lands.
pub fn available_languages() -> Vec<LanguageIdentifier> {
    LANGUAGE_LOADER
        .available_languages(&Localizations)
        .unwrap_or_default()
}

/// Runtime key lookup. Prefer the [`fl!`][crate::fl] macro when the key is a
/// literal — it validates the key against the `.ftl` files at compile time.
/// Use this only when the key is known at runtime (e.g. menu labels stored in
/// a `const` table that can't call the macro).
pub fn lookup(key: &str) -> String {
    LANGUAGE_LOADER.get(key)
}
