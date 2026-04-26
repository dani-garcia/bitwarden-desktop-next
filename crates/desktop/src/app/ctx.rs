//! Context bundles injected into view `update()` and `view()` calls, plus
//! the [`Outcome`] return type that every view `update()` produces.
//!
//! - [`UpdateCtx`] — for `update()`. Exposes SDK + session + the app-level
//!   overlay cell. Components MUST NOT receive it — rendering never spawns
//!   SDK work. The layer boundary lives in the type system: `components/`
//!   cannot reach `UpdateCtx`.
//! - [`RenderCtx`] — for `view()`. Render-time state (theme colors, favicon
//!   service, active user/email/accounts, layout hints, currently-open
//!   overlay). Components may receive it.
//! - [`Overlay`] — names every app-level overlay. Single source of truth so
//!   opening one replaces the previous by construction. Login vs. vault
//!   account switcher share a variant because only one view is visible at a
//!   time (mutex via `Screen`).
//! - [`Outcome`] — view `update()` return shape. `None` / `Task(t)` /
//!   `Event(e)`. The "both task and event" case is excluded — enrich the
//!   event so App's handler fires both effects.

use std::sync::Arc;

use iced::Task;

use crate::{
    components::sidebar::{SendFilter, VaultFilter},
    domain::UserId,
    services::{
        favicon::FaviconService,
        sdk::{AccountEntry, ClientManager},
    },
    theme::AppColors,
};

/// App-level overlays with mutual-exclusion semantics: at most one open at a
/// time. Cipher form field dropdowns are intentionally not represented here
/// — they're local to the form and don't conflict geometrically with these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// Submenus are single-level; the submenu index refers to a position
    /// inside `MENUS[menu].entries`.
    TitleBarMenu {
        menu: usize,
        submenu: Option<usize>,
    },
    /// Disambiguated between login and vault by the currently-rendered `Screen`.
    AccountSwitcher,
    ServerSelector,
}

/// Services + session passed to each view's `update()`. Rebuilt per call;
/// moved in by value because it holds `&mut` to App state (`open_overlay`).
pub struct UpdateCtx<'a> {
    pub client_manager: &'a Arc<ClientManager>,
    pub active_user: Option<&'a UserId>,
    /// Pulled from the App-level sidebar state; views need it on search-input
    /// changes, list reloads, etc.
    pub active_vault_filter: VaultFilter,
    pub active_send_filter: SendFilter,
    /// Writing here auto-closes whatever was open before (single-cell
    /// mutual-exclusion).
    pub open_overlay: &'a mut Option<Overlay>,
}

/// Render-time context passed to each view's `view()` and view-internal render
/// helpers. Also the only bundle a component may accept as a parameter.
pub struct RenderCtx<'a> {
    pub colors: &'a AppColors,
    pub favicon: &'a FaviconService,
    pub show_favicons: bool,
    pub window_width: f32,
    pub active_user: Option<&'a UserId>,
    pub active_email: Option<&'a str>,
    pub accounts: &'a [AccountEntry],
    pub open_overlay: Option<Overlay>,
}

/// Ties each view to its own Message + Event types so [`Outcome`] takes a
/// single type parameter — `Outcome<Self>` instead of
/// `Outcome<LoginMessage, LoginEvent>` at every handler signature.
pub trait ViewTypes {
    type Message: 'static;
    type Event;
}

/// The return shape for a view's `update()`. Three mutually-exclusive cases:
///
/// - [`Outcome::None`] — the common case, nothing bubbles out.
/// - [`Outcome::Task(t)`] — an async SDK call was spawned; wait for its
///   completion message.
/// - [`Outcome::Event(e)`] — a declarative fact for App to route.
///
/// Use `event.into()` or `Outcome::spawn(...)` at call sites. App routes a
/// view's `Outcome` via [`Outcome::dispatch`].
///
/// The "both task and event" case is deliberately excluded — views that need
/// to trigger "refresh list + push toast" emit a richer event (e.g.
/// `VaultEvent::ItemSaved`) and App's handler fires both effects.
pub enum Outcome<V: ViewTypes> {
    None,
    Task(Task<V::Message>),
    Event(V::Event),
}

impl<V: ViewTypes> Outcome<V> {
    /// Named constructor because a blanket `From<V::Event>` impl would
    /// conflict with stdlib's reflexive `From<T> for T`.
    pub fn event(event: V::Event) -> Self {
        Self::Event(event)
    }

    /// Escape hatch for pre-built tasks (`Task::batch`, `Task::done`, widget
    /// operations). Prefer [`Outcome::spawn`] for the async-block pattern.
    pub fn task(task: Task<V::Message>) -> Self {
        Self::Task(task)
    }

    /// `Some(e)` → `Event(e)`, `None` → `None`.
    pub fn from_option(event: Option<V::Event>) -> Self {
        match event {
            Some(e) => Self::Event(e),
            None => Self::None,
        }
    }
}

impl<V: ViewTypes> Outcome<V>
where
    V::Message: Send + 'static,
{
    /// Shorthand for `Outcome::task(Task::perform(future, on_complete))`.
    pub fn spawn<T: Send + 'static>(
        future: impl std::future::Future<Output = T> + Send + 'static,
        on_complete: impl Fn(T) -> V::Message + Send + 'static,
    ) -> Self {
        Self::Task(Task::perform(future, on_complete))
    }

    /// App-side router: lifts the view's `Message` via `wrap` and routes an
    /// event through `handle_event`, returning a single `Task` for App to
    /// schedule.
    pub fn dispatch<TopMsg: Send + 'static>(
        self,
        wrap: fn(V::Message) -> TopMsg,
        handle_event: impl FnOnce(V::Event) -> Task<TopMsg>,
    ) -> Task<TopMsg> {
        match self {
            Self::None => Task::none(),
            Self::Task(t) => t.map(wrap),
            Self::Event(e) => handle_event(e),
        }
    }
}
