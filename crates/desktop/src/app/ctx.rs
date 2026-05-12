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
    /// True when `uid` matches the currently-active user. Used by SDK
    /// completion handlers to drop stale results after an account switch.
    pub fn is_active_user(&self, uid: &UserId) -> bool {
        self.active_user == Some(uid)
    }

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
        V: View,
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
        V: View,
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
    /// Server URL for the active account, or `""` when no account is
    /// active or the entry is missing.
    pub active_server_url: &'a str,
    pub accounts: &'a [AccountEntry],
    pub open_overlay: Option<Overlay>,
}

/// The MVU contract every view implements: owns its own state (`Self`),
/// names its Message + Event types, and provides update + render methods.
///
/// `update` is expected to be pure with respect to `(self, msg, ctx)`.
/// Observable side effects (SDK calls, navigation, clipboard) must leave
/// via [`Outcome::Event`] or [`Outcome::Task`] so the App orchestrator
/// owns scheduling. Logging and animation timestamps are the two known
/// exceptions; everything else routes through `Outcome`.
///
/// Rendering is split into:
/// - [`Self::should_render`] — predicate gating whether [`Self::view`] should
///   be called this frame. Modal views return `false` when fully closed so
///   their dialog drops out of the tree entirely; full-screen views inherit
///   the default `true`.
/// - [`Self::view`] — the view's primary render surface. Always returns an
///   `Element`; callers must check [`Self::should_render`] first.
/// - [`Self::overlays`] — secondary surfaces stacked on top of the primary.
///   Sub-modals, bottom sheets, in-view confirmations. Each view manages
///   its own visibility internally and only pushes elements that should
///   render this frame.
pub trait View {
    type Message: 'static;
    type Event;

    fn update(&mut self, msg: Self::Message, ctx: UpdateCtx<'_>) -> Outcome<Self>;

    /// True when [`Self::view`] should be called this frame. Default `true`
    /// (full-screen views always render). Modal views override to gate on
    /// their `FadeInOut` visibility.
    fn should_render(&self) -> bool {
        true
    }

    /// Primary render surface. Caller must check [`Self::should_render`]
    /// first — implementations may assume that and panic / misrender if
    /// it's false.
    fn view<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
    ) -> iced::Element<'a, Self::Message, crate::theme::AppTheme>;

    /// Secondary surfaces stacked on top of [`Self::view`] in z-order
    /// (first element = lowest layer). Sub-modals, bottom sheets,
    /// confirmations. Returns an empty `Vec` by default.
    fn overlays<'a>(
        &'a self,
        _ctx: &RenderCtx<'a>,
    ) -> Vec<iced::Element<'a, Self::Message, crate::theme::AppTheme>> {
        Vec::new()
    }
}

/// App-level composition helpers on top of [`View`]. Lets the renderer
/// write `view.push_into(...)` directly instead of threading every view
/// through a free function. Blanket-implemented for every `View`.
///
/// The target message type `M` is inferred from the sink, and the
/// `M: From<Self::Message>` bound replaces a per-view fn pointer — each
/// view just needs `impl From<XxxMessage> for app::Message` (one line
/// each, defined alongside `Message`).
pub trait ViewExt: View {
    /// Push `view()` (gated by [`View::should_render`]) and `overlays()`,
    /// mapping each element to `M` via [`From`]. Use this for modal views
    /// where `view()` *is* an overlay (Settings, NewFolder, etc.).
    fn push_into<'a, M>(
        &'a self,
        ctx: &RenderCtx<'a>,
        out: &mut Vec<iced::Element<'a, M, crate::theme::AppTheme>>,
    ) where
        M: From<Self::Message> + 'static,
    {
        if self.should_render() {
            out.push(self.view(ctx).map(M::from));
        }
        for el in self.overlays(ctx) {
            out.push(el.map(M::from));
        }
    }

    /// Push only `overlays()`. Use this for full-screen views whose
    /// `view()` is rendered separately as page content (vault, send,
    /// login) — only their sub-modals belong in the overlay stack.
    fn push_overlays_into<'a, M>(
        &'a self,
        ctx: &RenderCtx<'a>,
        out: &mut Vec<iced::Element<'a, M, crate::theme::AppTheme>>,
    ) where
        M: From<Self::Message> + 'static,
    {
        for el in self.overlays(ctx) {
            out.push(el.map(M::from));
        }
    }
}

impl<V: View> ViewExt for V {}

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
pub enum Outcome<V: View + ?Sized> {
    None,
    Task(Task<V::Message>),
    Event(V::Event),
    Toast(Toast),
}

impl<V: View> Outcome<V> {
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

impl<V: View> Outcome<V>
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

    /// App-side router: lifts the view's `Message` into `TopMsg` via
    /// [`From`], dispatches events through `handle_event`, and forwards
    /// toasts via the [`PushToast`] impl on `state`. The `state`
    /// reference threads `&mut App` through the closure and the toast
    /// push so callers don't have to capture it twice (which would
    /// borrow-check as overlapping `&mut self`).
    ///
    /// `TopMsg` is typically `app::Message`; each view's local message
    /// converts via the `impl From<XxxMessage> for Message` defined
    /// alongside the message enum.
    pub fn dispatch<S, TopMsg, F>(self, state: &mut S, handle_event: F) -> Task<TopMsg>
    where
        S: PushToast,
        TopMsg: From<V::Message> + Send + 'static,
        F: FnOnce(&mut S, V::Event) -> Task<TopMsg>,
    {
        match self {
            Self::None => Task::none(),
            Self::Task(t) => t.map(Into::into),
            Self::Event(e) => handle_event(state, e),
            Self::Toast(t) => {
                state.push_toast(t);
                Task::none()
            }
        }
    }
}

/// Toast routing contract for the state type that drives [`Outcome::dispatch`].
/// `App` is the only impl in practice; the trait exists so `Outcome` stays
/// decoupled from concrete app state.
pub trait PushToast {
    fn push_toast(&mut self, toast: Toast);
}
