//! Shared helpers for UI tests — snapshots (Layer C/D), view-update unit
//! tests (Layer B), Simulator interaction tests (Layer C). Layer A (pure
//! helpers) needs nothing from here.
//!
//! ## Snapshot tests
//!
//! ```ignore
//! #[test]
//! fn fingerprint_modal() {
//!     test_support::init();
//!     let mut state = FingerprintModal::default();
//!     state.open_with("apple banana carrot dolphin eagle".to_owned());
//!     test_support::settle_animations();
//!
//!     for (theme, suffix) in [
//!         (AppTheme::light(), "light"),
//!         (AppTheme::dark(), "dark"),
//!     ] {
//!         let element = modal_view(&state, &theme.colors).expect("modal renders");
//!         test_support::assert_snapshot(
//!             format!("tests/snapshots/fingerprint_modal_{suffix}"),
//!             &theme,
//!             element,
//!         );
//!     }
//! }
//! ```
//!
//! Baselines land at `tests/snapshots/{name}-tiny-skia.png` — `iced_test`
//! appends `-{renderer_name}` before the extension. First run writes the
//! baseline, subsequent runs do an exact-byte compare. Delete the PNG to
//! re-baseline a view.
//!
//! ## View-update unit tests
//!
//! ```ignore
//! #[test]
//! fn submit_with_empty_name_is_ignored() {
//!     let mut view = NewFolderView::new();
//!     let mut owned = test_support::TestUpdateCtx::default();
//!     let out = view.update(NewFolderMessage::Submit, owned.as_ctx());
//!     assert!(matches!(out, Outcome::None));
//! }
//! ```
//!
//! ## Simulator interaction tests
//!
//! Simulator runs no runtime — Tasks don't execute, subscriptions don't
//! fire. Pattern is: render, drive widget events, drain produced messages,
//! apply them to your state by hand.
//!
//! ```ignore
//! #[test]
//! fn close_button_emits_close() {
//!     test_support::init();
//!     let mut state = FingerprintModal::default();
//!     state.open_with("phrase".into());
//!     test_support::settle_animations();
//!     let colors = AppTheme::light().colors;
//!     let element = modal_view(&state, &colors).unwrap();
//!     let mut ui = test_support::simulator(element);
//!     ui.click_text("Close").unwrap();
//!     let messages: Vec<_> = ui.into_messages().collect();
//!     assert!(messages
//!         .iter()
//!         .any(|m| matches!(m, FingerprintMessage::Close)));
//! }
//! ```
//!
//! Tests that need a `RenderCtx` (i.e. call a view's full `modal_view(&self,
//! &RenderCtx)`) build one via [`TestRenderCtx::default`] + [`TestRenderCtx::as_ctx`].
//! Constructing the default needs a tokio runtime — mark such tests
//! `#[tokio::test(flavor = "current_thread")]`. The favicon field is held
//! but unused for views that don't render row icons.

pub mod fixtures;

use std::{path::Path, sync::Arc, sync::OnceLock, thread, time::Duration};

use iced::{Element, Settings};
use iced_test::simulator::Simulator;

use crate::{
    APP_FONT,
    app::{Outcome, Overlay, RenderCtx, UpdateCtx, ViewTypes},
    assets,
    components::{icons, toast::Toast},
    domain::UserId,
    services::{
        favicon::FaviconService,
        sdk::{AccountEntry, ClientManager},
    },
    theme::{AppColors, AppTheme},
    views::{send::SendFilter, vault::VaultFilter},
};

/// Force the headless renderer backend to `tiny-skia` so snapshot bytes are
/// reproducible across machines (wgpu output drifts by GPU driver / mesa
/// version). Idempotent and parallel-safe via `OnceLock`.
pub fn init() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        // SAFETY: `set_var` is unsafe under the 2024 edition because other
        // threads may be reading the environment. Running this inside
        // `OnceLock::get_or_init` keeps the mutation single-shot and
        // ordered-before any test that reads `ICED_TEST_BACKEND`.
        unsafe {
            std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
        }
    });
}

/// Sleep past the default `FadeInOut` duration + jitter buffer so any
/// overlay-style view captures its fully-opened frame instead of a
/// mid-animation one.
pub fn settle_animations() {
    thread::sleep(Duration::from_millis(300));
}

/// Build a [`Simulator`] with the app's production fonts loaded so text and
/// icon glyphs render identically to the running binary.
pub fn simulator<'a, M>(
    element: impl Into<Element<'a, M, AppTheme>>,
) -> Simulator<'a, M, AppTheme> {
    let settings = Settings {
        fonts: vec![
            assets::FONT_MEDIUM.into(),
            assets::FONT_BOLD.into(),
            assets::BWI_FONT.into(),
            icons::FONT_BYTES.into(),
        ],
        default_font: APP_FONT,
        ..Settings::default()
    };
    Simulator::with_settings(settings, element)
}

/// Render the element with the given theme and compare against the PNG
/// baseline at `tests/snapshots/{path}-{renderer}.png`. Writes the baseline
/// on first run; panics on later runs if the bytes drift.
pub fn assert_snapshot<'a, M>(
    path: impl AsRef<Path>,
    theme: &AppTheme,
    element: impl Into<Element<'a, M, AppTheme>>,
) {
    let path = path.as_ref();
    let mut ui = simulator(element);
    let snapshot = ui.snapshot(theme).expect("rendered snapshot");
    let matched = snapshot
        .matches_image(path)
        .expect("snapshot comparison");
    assert!(
        matched,
        "snapshot drift at {}; delete the .png to re-baseline",
        path.display(),
    );
}

/// Owns the storage a [`UpdateCtx`]'s `&mut` slots borrow from, so tests can
/// use `Default` and customize fields by struct-update syntax.
///
/// `UpdateCtx<'a>` itself can't implement `Default` — it holds `&mut`
/// references with a borrow lifetime, and `Default::default()` would have to
/// conjure storage from nowhere (forcing leaks or `'static` shared state).
/// This wrapper holds the storage by value and yields a fresh borrow via
/// [`Self::as_ctx`].
///
/// ```ignore
/// let mut owned = TestUpdateCtx::default();
/// let out = view.update(msg, owned.as_ctx());
///
/// // Override a field via struct-update:
/// let mut owned = TestUpdateCtx {
///     active_vault_filter: VaultFilter::Trash,
///     ..Default::default()
/// };
/// ```
pub struct TestUpdateCtx {
    pub client_manager: ClientManager,
    pub open_overlay: Option<Overlay>,
    pub active_user: Option<UserId>,
    pub active_vault_filter: VaultFilter,
    pub active_send_filter: SendFilter,
}

impl Default for TestUpdateCtx {
    fn default() -> Self {
        Self {
            client_manager: ClientManager::empty(),
            open_overlay: None,
            active_user: None,
            active_vault_filter: VaultFilter::AllItems,
            active_send_filter: SendFilter::AllItems,
        }
    }
}

impl TestUpdateCtx {
    /// Build a fresh [`UpdateCtx`] borrowing from this owned storage. Call
    /// once per `view.update(...)` invocation — the borrow can't outlive
    /// the surrounding statement.
    pub fn as_ctx(&mut self) -> UpdateCtx<'_> {
        UpdateCtx {
            client_manager: &mut self.client_manager,
            active_user: self.active_user.as_ref(),
            active_vault_filter: self.active_vault_filter,
            active_send_filter: self.active_send_filter,
            open_overlay: &mut self.open_overlay,
        }
    }
}

/// Owns the storage a [`RenderCtx`]'s `&` slots borrow from. Same shape as
/// [`TestUpdateCtx`]: `Default::default()` then `.as_ctx()`.
///
/// Constructing the default panics outside a tokio runtime because
/// [`FaviconService::new`] captures the runtime handle — mark tests
/// `#[tokio::test(flavor = "current_thread")]`. The favicon service is held
/// but unused unless a view actively calls `.get()`, so this is fine for
/// most views.
pub struct TestRenderCtx {
    pub colors: AppColors,
    pub favicon: FaviconService,
    pub show_favicons: bool,
    pub window_width: f32,
    pub active_user: Option<UserId>,
    pub active_email: Option<String>,
    pub active_server_url: String,
    pub accounts: Vec<AccountEntry>,
    pub open_overlay: Option<Overlay>,
}

impl Default for TestRenderCtx {
    fn default() -> Self {
        Self {
            colors: AppTheme::light().colors,
            favicon: FaviconService::new(Arc::new(|_| String::new())),
            show_favicons: false,
            window_width: 800.0,
            active_user: None,
            active_email: None,
            active_server_url: String::new(),
            accounts: Vec::new(),
            open_overlay: None,
        }
    }
}

impl TestRenderCtx {
    pub fn as_ctx(&self) -> RenderCtx<'_> {
        RenderCtx {
            colors: &self.colors,
            favicon: &self.favicon,
            show_favicons: self.show_favicons,
            window_width: self.window_width,
            active_user: self.active_user.as_ref(),
            active_email: self.active_email.as_deref(),
            active_server_url: &self.active_server_url,
            accounts: &self.accounts,
            open_overlay: self.open_overlay,
        }
    }
}

/// Test-only extension methods for asserting on a view's [`Outcome`].
///
/// `Outcome` can't derive `PartialEq` (Task is opaque) and the contained
/// types vary per view, so most tests fall back to `match` blocks with a
/// `panic!` arm. These helpers compress that to a single method call:
///
/// ```ignore
/// let ev = view.update(msg, owned.as_ctx()).expect_event();
/// assert!(matches!(ev, NewFolderEvent::Run(s) if s == "Social"));
/// ```
pub trait OutcomeExt<V: ViewTypes> {
    /// Panic unless the outcome is [`Outcome::None`].
    fn expect_none(self);
    /// Panic unless the outcome is [`Outcome::Event`]; return the event.
    fn expect_event(self) -> V::Event;
    /// Panic unless the outcome is [`Outcome::Toast`]; return the toast.
    fn expect_toast(self) -> Toast;
}

impl<V: ViewTypes> OutcomeExt<V> for Outcome<V> {
    fn expect_none(self) {
        if !matches!(self, Outcome::None) {
            panic!("expected Outcome::None, got {}", outcome_kind(&self));
        }
    }

    fn expect_event(self) -> V::Event {
        match self {
            Outcome::Event(e) => e,
            other => panic!("expected Outcome::Event, got {}", outcome_kind(&other)),
        }
    }

    fn expect_toast(self) -> Toast {
        match self {
            Outcome::Toast(t) => t,
            other => panic!("expected Outcome::Toast, got {}", outcome_kind(&other)),
        }
    }
}

fn outcome_kind<V: ViewTypes>(out: &Outcome<V>) -> &'static str {
    match out {
        Outcome::None => "Outcome::None",
        Outcome::Task(_) => "Outcome::Task(_)",
        Outcome::Event(_) => "Outcome::Event(_)",
        Outcome::Toast(_) => "Outcome::Toast(_)",
    }
}

/// Layer-C harness: wrap an element in a `Simulator`, drive interactions
/// against it, drain the produced messages. Build the element inline at
/// the call site — that's where Rust can reason about element-borrows-state
/// + element-borrows-context lifetimes naturally.
///
/// ```ignore
/// let element = view.modal_view(&render.as_ctx()).expect("modal renders");
/// let messages = drive_element(element, |ui| {
///     ui.click("Save").expect("Save button");
/// });
/// ```
pub fn drive_element<'a, M>(
    element: impl Into<Element<'a, M, AppTheme>>,
    interact: impl FnOnce(&mut Simulator<'a, M, AppTheme>),
) -> Vec<M>
where
    M: 'static,
{
    let mut ui = simulator(element);
    interact(&mut ui);
    ui.into_messages().collect()
}
