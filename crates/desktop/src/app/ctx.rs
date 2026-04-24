//! Context bundles injected into view `update()` and `view()` calls, plus
//! the [`Outcome`] return type that every view `update()` produces.
//!
//! - [`UpdateCtx`] is built by [`App::update`] and passed to each view's
//!   `update()`. It exposes services (SDK) + session info a view legitimately
//!   needs to spawn tasks or run logic, plus the app-level overlay cell a
//!   view toggles to open or close its dropdown. Components MUST NOT receive
//!   it — rendering never spawns SDK work.
//! - [`RenderCtx`] is built by [`App::view`] and passed to each view's
//!   `view()`. It exposes render-time state: theme colors, favicon service,
//!   active user/email/accounts, layout hints, and the currently-open
//!   overlay. Components are allowed to receive it.
//! - [`Overlay`] names every app-level overlay. App holds `Option<Overlay>`
//!   as the single source of truth so opening any overlay replaces the
//!   previous one by construction — no explicit dismissal needed. The login
//!   vs. vault account switcher use the same variant because only one view
//!   is visible at a time (mutex via `Screen`).
//! - [`Outcome`] is the return shape for view `update()`. Mutually exclusive
//!   variants: `None` / `Task(t)` / `Event(e)`. When a view wants both a
//!   task and user-visible feedback (e.g. "save succeeded"), enrich the
//!   event so App's handler fires both effects.
//!
//! The layer boundary lives in the type system: `components/` cannot reach
//! `UpdateCtx`, so a component physically cannot call `client_manager.foo()`.

use std::sync::Arc;

use iced::Task;

use crate::{
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
    /// Title-bar menu with an optionally-open submenu. Submenus are single-
    /// level (one item from the top-level menu), so the submenu index refers
    /// to a position inside `MENUS[menu].entries`.
    TitleBarMenu { menu: usize, submenu: Option<usize> },
    /// Account switcher dropdown, shown on either the login or vault view.
    /// Disambiguated by the currently-rendered `Screen`.
    AccountSwitcher,
    /// Server selector popover on the login-email page.
    ServerSelector,
}

/// Services + session passed to each view's `update()`. Rebuilt per call;
/// moved in by value because it holds `&mut` to App state (`open_overlay`).
pub struct UpdateCtx<'a> {
    pub client_manager: &'a Arc<ClientManager>,
    pub active_user: Option<&'a UserId>,
    /// The single cell holding the currently-open app-level overlay. Views
    /// toggle their dropdowns by writing `Some(Overlay::X)` / `None` here —
    /// writing a new value automatically closes whatever was open before.
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

/// Ties each view to its own Message + Event types. Implementations live
/// alongside the view (one-liner `impl ViewTypes for LoginView { ... }`).
/// Existence of this trait lets [`Outcome`] take a single type parameter —
/// `Outcome<Self>` at every handler signature instead of
/// `Outcome<LoginMessage, LoginEvent>`.
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
    /// Constructor for event-producing arms. A blanket `From<V::Event>`
    /// impl would conflict with stdlib's reflexive `From<T> for T` (when
    /// `V::Event = Outcome<V>` hypothetically), so call sites use this
    /// named constructor instead.
    pub fn event(event: V::Event) -> Self {
        Self::Event(event)
    }

    /// Constructor for task-producing arms. Today every call site wraps
    /// an `async` block and uses [`Outcome::spawn`] — this escape hatch
    /// exists for pre-built tasks (`Task::batch`, `Task::done`, etc.)
    /// when a future feature needs them.
    #[expect(
        dead_code,
        reason = "API completeness: escape hatch for pre-built Tasks"
    )]
    pub fn task(task: Task<V::Message>) -> Self {
        Self::Task(task)
    }

    /// Lift an `Option<Event>` into an `Outcome` — `Some(e)` becomes
    /// `Event(e)`, `None` becomes `None`. Use when a lookup chain yields
    /// an optional event: `Outcome::from_option(maybe_event)`.
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
    /// Shorthand for spawning a future + producing the completion message.
    /// Equivalent to `Outcome::task(Task::perform(future, on_complete))`
    /// but hides one layer of iced-specific wrapping at the call site.
    pub fn spawn<T: Send + 'static>(
        future: impl std::future::Future<Output = T> + Send + 'static,
        on_complete: impl Fn(T) -> V::Message + Send + 'static,
    ) -> Self {
        Self::Task(Task::perform(future, on_complete))
    }

    /// App-side router helper. Lifts a view's local `Message` type into the
    /// top-level `Message` via `wrap`, and routes an event through
    /// `handle_event`. Returns the combined `Task` for App to schedule.
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
