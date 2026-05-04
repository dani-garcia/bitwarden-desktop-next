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

use std::future::Future;

use bitwarden_pm::PasswordManagerClient;
use iced::Task;

use crate::{
    components::toast::Toast,
    domain::UserId,
    services::{
        favicon::FaviconService,
        sdk::{AccountEntry, ClientManager},
    },
    theme::AppColors,
    views::{send::SendFilter, vault::VaultFilter},
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
    /// Vault header "+New" button → cipher-type picker.
    NewItemMenu,
}

/// Services + session passed to each view's `update()`. Rebuilt per call;
/// moved in by value because it holds `&mut` to App state (`open_overlay`).
pub struct UpdateCtx<'a> {
    /// `&mut` so views can call sync mutating methods (`save_send`,
    /// `push_history`, …). Async SDK work runs on a `client_for(uid)` handle
    /// captured at the start of the task; the manager itself is never
    /// borrowed across `.await`.
    pub client_manager: &'a mut ClientManager,
    pub active_user: Option<&'a UserId>,
    /// Pulled from the App-level sidebar state; views need it on search-input
    /// changes, list reloads, etc.
    pub active_vault_filter: VaultFilter,
    pub active_send_filter: SendFilter,
    /// Writing here auto-closes whatever was open before (single-cell
    /// mutual-exclusion).
    pub open_overlay: &'a mut Option<Overlay>,
}

impl UpdateCtx<'_> {
    /// View-side mirror of `App::perform_with_active_client`. Extracts the
    /// active user's `PasswordManagerClient`, runs an async call against it,
    /// and wraps the result `Task` in `Outcome::Task` for direct return from
    /// a view's `update` arm. `Outcome::None` if no active user or the user
    /// isn't loaded.
    pub fn perform_with_active_client<V, Spawn, Fut, T>(
        &self,
        spawn: Spawn,
        on_complete: impl Fn(UserId, T) -> V::Message + Send + 'static,
    ) -> Outcome<V>
    where
        V: ViewTypes,
        V::Message: Send + 'static,
        Spawn: FnOnce(PasswordManagerClient) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let Some(uid) = self.active_user.copied() else {
            return Outcome::None;
        };
        self.perform_with_client(uid, spawn, move |t| on_complete(uid, t))
    }

    /// Variant of [`Self::perform_with_active_client`] for callers that
    /// already have a specific uid in hand (e.g. captured from a message
    /// payload). Same defensive `Outcome::None` bail when the user isn't
    /// loaded.
    pub fn perform_with_client<V, Spawn, Fut, T>(
        &self,
        uid: UserId,
        spawn: Spawn,
        on_complete: impl Fn(T) -> V::Message + Send + 'static,
    ) -> Outcome<V>
    where
        V: ViewTypes,
        V::Message: Send + 'static,
        Spawn: FnOnce(PasswordManagerClient) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let Some(client) = self.client_manager.client_for(&uid) else {
            return Outcome::None;
        };
        Outcome::Task(Task::perform(spawn(client), on_complete))
    }
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

/// The return shape for a view's `update()`. Mutually-exclusive cases:
///
/// - [`Outcome::None`] — the common case, nothing bubbles out.
/// - [`Outcome::Task(t)`] — an async SDK call was spawned; wait for its
///   completion message.
/// - [`Outcome::Event(e)`] — a declarative fact for App to route.
/// - [`Outcome::Toast(t)`] — a toast to surface. Routed by [`dispatch`]
///   directly so views don't need a `ToastRequested` event variant just to
///   forward it.
///
/// Use `event.into()` or `Outcome::perform(...)` at call sites. App routes a
/// view's `Outcome` via [`Outcome::dispatch`].
///
/// The "both task and event" case is deliberately excluded — views that need
/// to trigger "refresh list + push toast" emit a richer event (e.g.
/// `VaultEvent::ItemSaved`) and App's handler fires both effects.
pub enum Outcome<V: ViewTypes> {
    None,
    Task(Task<V::Message>),
    Event(V::Event),
    Toast(Toast),
}

impl<V: ViewTypes> Outcome<V> {
    /// Named constructor because a blanket `From<V::Event>` impl would
    /// conflict with stdlib's reflexive `From<T> for T`.
    pub fn event(event: V::Event) -> Self {
        Self::Event(event)
    }

    /// Escape hatch for pre-built tasks (`Task::batch`, `Task::done`, widget
    /// operations). Prefer [`Outcome::perform`] for the async-block pattern.
    pub fn task(task: Task<V::Message>) -> Self {
        Self::Task(task)
    }

    /// Surface a toast. Routed directly by [`Outcome::dispatch`] — views
    /// don't need a `ToastRequested` event variant just to forward it.
    pub fn toast(toast: Toast) -> Self {
        Self::Toast(toast)
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
    pub fn perform<T: Send + 'static>(
        future: impl std::future::Future<Output = T> + Send + 'static,
        on_complete: impl Fn(T) -> V::Message + Send + 'static,
    ) -> Self {
        Self::Task(Task::perform(future, on_complete))
    }

    /// App-side router: lifts the view's `Message` via `wrap`, dispatches
    /// events through `handle_event`, and forwards toasts directly via
    /// `push_toast`. The `state` reference threads `&mut App` through both
    /// closures so callers don't have to capture it twice (which would
    /// borrow-check as overlapping `&mut self`).
    pub fn dispatch<S, TopMsg: Send + 'static, F>(
        self,
        state: &mut S,
        wrap: fn(V::Message) -> TopMsg,
        handle_event: F,
        push_toast: fn(&mut S, Toast),
    ) -> Task<TopMsg>
    where
        F: FnOnce(&mut S, V::Event) -> Task<TopMsg>,
    {
        match self {
            Self::None => Task::none(),
            Self::Task(t) => t.map(wrap),
            Self::Event(e) => handle_event(state, e),
            Self::Toast(t) => {
                push_toast(state, t);
                Task::none()
            }
        }
    }
}
