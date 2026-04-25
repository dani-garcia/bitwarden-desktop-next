mod ctx;
mod handlers;
mod helpers;
mod message;
mod window;

pub use ctx::{Outcome, Overlay, RenderCtx, UpdateCtx, ViewTypes};
pub use message::{Message, SystemMessage, ViewMessage, WindowMessage};
use window::{WindowInfo, WindowKind};

use std::{collections::HashMap, rc::Rc, sync::Arc};

use iced::{Element, Subscription, Task};

use crate::{
    components::{
        sidebar::{self, SidebarState},
        toast::{self, Toast},
    },
    domain::{Screen, UserId},
    services::{
        clipboard::ClipboardManager,
        sdk::{AccountEntry, ClientManager},
        settings::Settings,
        tray::TrayHandle,
    },
    theme::{AppTheme, ThemePreference},
    views::{
        generator as generator_view, login, magnify, send, settings as settings_view,
        title_bar::{self, TitleBarMessage},
        vault,
    },
};

use helpers::main_window_platform_specific;

pub struct App {
    // ── Domain state ───────────────────────────────────────────────────────
    pub(super) active_user: Option<UserId>,

    // ── Sub-views (each owns its own state + async lifecycle) ──────────────
    pub(super) views: Views,

    // ── External dependencies ──────────────────────────────────────────────
    pub(super) client_manager: Arc<ClientManager>,
    pub(super) favicon: crate::services::favicon::FaviconService,

    // ── Derived cache ──────────────────────────────────────────────────────
    // Exists because iced's `view()` returns an `Element<'_, ...>` that
    // borrows from `&self`; locally-computed Vecs inside `view()` would be
    // dropped before the Element. Cached fields survive the borrow. See
    // `docs/architecture.md` → "Why Things Are the Way They Are".
    pub(super) cache: ViewCache,

    // ── Window chrome ─────────────────────────────────────────────────────
    pub(super) screen: Screen,
    /// App-level sidebar state. Persists across screen transitions so the
    /// user's collapse / filter selection isn't reset when navigating
    /// between Vault and Send.
    pub(super) sidebar: SidebarState,
    pub(super) theme: ThemeState,
    pub(super) windows: HashMap<iced::window::Id, WindowInfo>,
    /// Cached id of the main window — always present from `App::new` until
    /// `iced::exit`. Child windows (About) are not tracked here.
    pub(super) main_window: iced::window::Id,
    pub(super) native_menu: Option<crate::services::menu::NativeMenuHandle>,

    // ── Tray + user settings ───────────────────────────────────────────────
    // `settings` is read once at startup from `data/settings.json`. A future
    // settings view mutates fields directly; all tray / lifecycle branches
    // re-read `self.settings.<field>` at event time (never cached), so live
    // changes apply without a restart.
    pub(super) settings: Settings,
    pub(super) tray: Option<TrayHandle>,

    // ── Cross-cutting UI overlay queue ─────────────────────────────────────
    pub(super) toasts: Vec<Toast>,

    // ── Single-overlay cell ────────────────────────────────────────────────
    // Source of truth for "which dropdown/menu is open right now". Views
    // write here through `UpdateCtx::open_overlay`; by construction at most
    // one overlay can be open, so opening one auto-dismisses any other.
    pub(super) open_overlay: Option<Overlay>,

    // ── Clipboard manager ──────────────────────────────────────────────────
    // Single writer: every clipboard `set` flows through here so the 30 s
    // auto-clear bookkeeping sees every write. See `clipboard.rs`.
    pub(super) clipboard: ClipboardManager,

    // ── Magnify launcher ──────────────────────────────────────────────────
    // Owns its own (lazily-opened) window; summoned by the global hotkey.
    pub(super) magnify: magnify::MagnifyView,
}

/// Derived data cached across `view()` rebuilds. Refreshed only by the
/// handlers that actually mutate the inputs (login, logout, lock, unlock,
/// user switch, manager load), not on every update cycle.
///
/// Exists because iced's `view()` returns an `Element<'_, ...>` that borrows
/// from `&self`, so the slice handed to `account_switcher::dropdown` has to
/// live on App, not be built inline in `view()`.
#[derive(Default)]
pub(super) struct ViewCache {
    pub accounts: Vec<AccountEntry>,
}

/// Sub-views grouped into their own struct so `App::update` can split its
/// borrow: the view-dispatch arm takes `&mut self.views` plus `&mut
/// self.open_overlay` via `UpdateCtx`, while event handlers reborrow other
/// App fields (client_manager, settings, etc.) without overlap.
pub struct Views {
    pub(super) login: login::LoginView,
    pub(super) vault: vault::VaultView,
    pub(super) send: send::SendView,
    pub(super) settings: settings_view::SettingsView,
    pub(super) generator: generator_view::GeneratorView,
    pub(super) title_bar: title_bar::TitleBarView,
}

impl Views {
    pub fn new() -> Self {
        Self {
            login: login::LoginView::new(),
            vault: vault::VaultView::new(),
            send: send::SendView::new(),
            settings: settings_view::SettingsView::new(),
            generator: generator_view::GeneratorView::new(),
            title_bar: title_bar::TitleBarView::new(),
        }
    }
}

/// Theme state bundle: user preference, resolved `AppTheme` instance,
/// and the OS observer we subscribe to for system-theme changes.
pub struct ThemeState {
    pub(super) preference: ThemePreference,
    pub current: AppTheme,
    pub(super) system: Rc<system_theme::SystemTheme>,
}

impl ThemeState {
    pub fn new(preference: ThemePreference) -> Self {
        let system = Rc::new(
            system_theme::SystemTheme::new().expect("failed to initialize system theme observer"),
        );
        let current = preference.resolve(system.get_scheme());
        Self {
            preference,
            current,
            system,
        }
    }

    pub fn refresh(&mut self) {
        if self.preference == ThemePreference::System {
            self.current = self.preference.resolve(self.system.get_scheme());
        }
    }
}

// ── Construction + iced daemon callbacks ───────────────────────────────────

pub(super) const MAIN_WINDOW_SIZE: iced::Size = iced::Size::new(1024.0, 800.0);

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let settings = Settings::load();
        let user_theme = settings.theme;

        // Apply the persisted language preference before any `fl!()` call
        // resolves user-visible strings. Empty string = "follow OS locale"
        // which the `i18n::init()` earlier in `main` already honoured.
        if !settings.language.is_empty()
            && let Ok(tag) = settings.language.parse()
        {
            crate::services::i18n::set_language(tag);
        }

        // Register muda + tray-icon event handlers into their global
        // broadcast channels. Must run *before* any menu or tray icon is
        // built — both crates' `set_event_handler` are one-shot.
        crate::services::menu::install_event_handler();
        crate::services::tray::install_event_handler();
        // Global keyboard shortcuts (currently just the Magnify hotkey).
        // Failure (e.g. Wayland) is logged inside `install_event_handler`
        // and the rest of the app keeps running — Magnify is just unreachable.
        crate::services::global_hotkey::install_event_handler();

        // `--autostart` (used by the future "open at login" path) forces the
        // app to launch hidden in the tray every time. We force-build the
        // tray in that case even if the user hasn't enabled any tray
        // settings, otherwise the hidden window would have no entry point.
        let autostart = std::env::args().any(|a| a == "--autostart");

        // Build the tray up-front if any tray-related setting is on, or if
        // `--autostart` requires it, so user clicks find it immediately.
        // Tray build failure → log + fall through without.
        let mut tray = None;
        if settings.wants_tray() || autostart {
            tray = crate::services::tray::build();
            if tray.is_none() {
                tracing::warn!("tray requested but failed to initialise");
            }
        }

        // Always open the main window; when `--autostart` is set (and the
        // tray actually initialised), open it hidden so the user sees only
        // the tray. Iced plumbs `visible: false` through winit's
        // `with_visible(false)` — the window never flashes on screen, iced
        // still owns the id and keeps firing `view()`. Toggling later is a
        // cheap `Mode::Windowed` / `gain_focus`.
        let start_hidden = autostart && tray.is_some();
        let (main_id, open_task) = iced::window::open(iced::window::Settings {
            size: MAIN_WINDOW_SIZE,
            min_size: Some(iced::Size::new(800.0, 750.0)),
            decorations: crate::services::menu::should_use_native_title_bar(),
            visible: !start_hidden,
            // Required for tray "close to tray": without this, the OS-sent
            // `CloseRequested` event would close the window before our
            // `WindowAction::Close` handler can choose to hide instead.
            // The About window keeps the default `true` (never hides).
            exit_on_close_request: false,
            platform_specific: main_window_platform_specific(),
            icon: iced::window::icon::from_file_data(
                crate::assets::ICON_PNG,
                Some(image::ImageFormat::Png),
            )
            .ok(),
            ..Default::default()
        });
        let mut windows = HashMap::new();
        windows.insert(main_id, WindowInfo::new(WindowKind::Main, MAIN_WINDOW_SIZE));

        // Discover users in `<workspace-root>/data/` and open one SQLite DB
        // per user. Runs as a regular async task on iced's tokio multi-thread
        // runtime; the `ClientManagerLoaded` handler swaps the Arc when done.
        let load_task = Task::perform(async { Arc::new(ClientManager::load().await) }, |mgr| {
            Message::System(SystemMessage::ClientManagerLoaded(mgr))
        });

        // Favicon service. Every user points at the cloud default for now;
        // see `docs/todo.md` "Show favicons" for the per-user `icons_url` swap.
        let favicon = crate::services::favicon::FaviconService::new(Arc::new(|_uid: &UserId| {
            "https://icons.bitwarden.net".to_string()
        }));

        let app = Self {
            active_user: None,
            screen: Screen::Loading,
            sidebar: SidebarState::default(),
            views: Views::new(),
            client_manager: Arc::new(ClientManager::empty()),
            favicon,
            cache: ViewCache::default(),
            theme: ThemeState::new(user_theme),
            windows,
            main_window: main_id,
            native_menu: None,
            settings,
            tray,
            toasts: Vec::new(),
            open_overlay: None,
            clipboard: ClipboardManager::new(),
            magnify: magnify::MagnifyView::new(),
        };

        (
            app,
            Task::batch([
                open_task.map(|id| Message::Window(WindowMessage::Opened(id))),
                load_task,
            ]),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Window close events — needed for daemon mode lifecycle. When the
        // main window closes we call `iced::exit()` to terminate the daemon;
        // closing a child window (About) just removes it from the map.
        let close_sub =
            iced::window::close_events().map(|id| Message::Window(WindowMessage::Closed(id)));

        // Keyboard events are routed through `event::listen_with` (not
        // `keyboard::listen`) so the `window::Id` is provided natively by
        // the listener. `Subscription::map` requires a non-capturing
        // closure (iced enforces this at const-eval time), and we need
        // the id bound into the emitted message for daemon-readiness.
        // The same listener also forwards `window::Event::Resized` so the
        // app can drive a responsive layout, `CloseRequested` so close-
        // to-tray can intercept OS-native close actions, and `Unfocused`
        // so the Magnify launcher can dismiss on click-outside.
        let event_sub = iced::event::listen_with(|event, _status, id| match event {
            iced::Event::Keyboard(ev) => Some(Message::Window(WindowMessage::KeyPressed(id, ev))),
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::Window(WindowMessage::Resized(id, size)))
            }
            iced::Event::Window(iced::window::Event::CloseRequested) => {
                Some(Message::Window(WindowMessage::CloseRequested(id)))
            }
            iced::Event::Window(iced::window::Event::Unfocused) => {
                Some(Message::Window(WindowMessage::Unfocused(id)))
            }
            _ => None,
        });

        // muda's `MenuEvent::receiver()` is global — the native app menu and
        // the tray context menu both dispatch to it. A single pump thread
        // (bound inside the stream) forwards events here; the handler filters
        // by `MenuId` to decide whether it's an app-menu or tray-menu click.
        let muda_sub = if self.native_menu.is_some() || self.tray.is_some() {
            Subscription::run(crate::services::menu::muda_event_stream)
                .map(|ev| Message::System(SystemMessage::MudaEvent(ev)))
        } else {
            Subscription::none()
        };

        // Tray icon left-click events come through a separate receiver. The
        // stream filters to the click-to-toggle case before emitting, so the
        // handler just runs the action.
        let tray_sub = if self.tray.is_some() {
            Subscription::run(crate::services::tray::click_stream)
                .map(|action| Message::System(SystemMessage::TrayClick(action)))
        } else {
            Subscription::none()
        };

        let theme_sub = Subscription::run_with(self.theme.system.clone(), |st| st.subscribe())
            .map(|_| Message::System(SystemMessage::ThemeChanged));

        // Second-launch wake-up: the listener is bound once, inside this
        // stream, and kept alive across `update()` cycles because iced
        // hashes the subscription identity from the `fn` pointer.
        let wake_sub = Subscription::run(crate::services::instance_lock::wake_stream)
            .map(|_| Message::System(SystemMessage::InstanceWakeRequested));

        // Global hotkey → Magnify launcher toggle. Idle when the OS-side
        // registration failed (Wayland, missing permissions) — the stream
        // terminates immediately and the subscription stays dormant.
        let magnify_sub = Subscription::run(crate::services::global_hotkey::event_stream)
            .map(|_| Message::Magnify(crate::views::magnify::MagnifyMessage::HotkeyPressed));

        // Favicon fetch completions. Each message flips one row from globe
        // to real icon by virtue of arriving; the handler just logs.
        let favicon_sub =
            Subscription::run(crate::services::favicon::favicon_event_stream).map(Message::Favicon);

        // Animation ticker — only subscribed while at least one transition
        // somewhere in the app might still be running. The check is a single
        // global-watermark read against `services::animation`, so any new
        // animation primitive that calls `animation::extend(...)` on its
        // transitions auto-registers here without extra wiring.
        let anim_sub = if crate::services::animation::any_in_progress() {
            iced::window::frames().map(|_| Message::AnimationTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([
            close_sub,
            event_sub,
            muda_sub,
            tray_sub,
            theme_sub,
            wake_sub,
            favicon_sub,
            anim_sub,
            magnify_sub,
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::About(m) => self.handle_about_message(m),
            Message::Window(m) => self.handle_window_message(m),
            Message::System(m) => self.handle_system_message(m),
            Message::Sidebar(m) => self.handle_sidebar_message(m),
            Message::Magnify(m) => self.handle_magnify_message(m),
            // No-op handler — the redraw triggered by this message reaching
            // update() is the only thing we need. Subscription rebuilds after
            // the redraw and unsubscribes once nothing's animating.
            Message::AnimationTick => Task::none(),
            Message::Favicon(crate::services::favicon::FaviconMessage::IconResolved {
                uid,
                hostname,
            }) => {
                // Arrival of this message is what drives the redraw; the
                // service has already mutated its in-memory cache by the
                // time we see it here. Log at trace so a cold unlock with
                // thousands of icons doesn't spam RUST_LOG=info users.
                tracing::trace!(%uid, %hostname, "favicon resolved");
                Task::none()
            }
            Message::View(view_msg) => {
                // One `UpdateCtx` shared across every view dispatch; its
                // `&mut self.open_overlay` borrow only needs to coexist with
                // the disjoint `&mut self.views.*` borrow below.
                let active_vault_filter = self.sidebar.active_vault_filter;
                let active_send_filter = self.sidebar.active_send_filter;
                let uctx = UpdateCtx {
                    client_manager: &self.client_manager,
                    active_user: self.active_user.as_ref(),
                    active_vault_filter,
                    active_send_filter,
                    open_overlay: &mut self.open_overlay,
                };
                match view_msg {
                    ViewMessage::Login(m) => self
                        .views
                        .login
                        .update(m, uctx)
                        .dispatch(Message::login, |e| self.handle_login_event(e)),
                    ViewMessage::Vault(m) => self
                        .views
                        .vault
                        .update(m, uctx)
                        .dispatch(Message::vault, |e| self.handle_vault_event(e)),
                    ViewMessage::Send(m) => self
                        .views
                        .send
                        .update(m, uctx)
                        .dispatch(Message::send, |e| self.handle_send_event(e)),
                    ViewMessage::TitleBar(m) => self
                        .views
                        .title_bar
                        .update(m, uctx)
                        .dispatch(Message::title_bar, |e| self.handle_titlebar_event(e)),
                    ViewMessage::Settings(m) => self
                        .views
                        .settings
                        .update(m, uctx)
                        .dispatch(Message::settings, |e| self.handle_settings_event(e)),
                    ViewMessage::Generator(m) => self
                        .views
                        .generator
                        .update(m, uctx)
                        .dispatch(Message::generator, |e| self.handle_generator_event(e)),
                }
            }
        }
    }

    pub fn view(&self, window_id: iced::window::Id) -> Element<'_, Message, AppTheme> {
        match self.windows.get(&window_id).map(|w| w.kind) {
            Some(WindowKind::Main) => self.view_main(),
            Some(WindowKind::About) => {
                crate::views::about::view(&self.theme.current.colors).map(Message::About)
            }
            Some(WindowKind::Magnify) => crate::views::magnify::view(
                &self.magnify,
                &self.favicon,
                self.settings.show_favicons,
                self.active_user.as_ref(),
                &self.theme.current.colors,
            )
            .map(Message::Magnify),
            // Defensive: all windows are eagerly inserted at creation time,
            // but if iced calls view for an id we somehow don't know about,
            // an empty space is a safe no-op.
            None => iced::widget::Space::new().into(),
        }
    }

    pub fn title(&self, window_id: iced::window::Id) -> String {
        match self.windows.get(&window_id).map(|w| w.kind) {
            Some(WindowKind::About) => crate::fl!("about-window-title"),
            _ => crate::fl!("app-title"),
        }
    }

    pub fn theme(&self, window_id: iced::window::Id) -> AppTheme {
        let theme = self.theme.current.clone();
        // Magnify launcher needs a transparent OS-window background so the
        // pixels outside its rounded container stay see-through.
        if matches!(
            self.windows.get(&window_id).map(|w| w.kind),
            Some(WindowKind::Magnify)
        ) {
            theme.with_transparent_background()
        } else {
            theme
        }
    }

    fn view_main(&self) -> Element<'_, Message, AppTheme> {
        let colors = &self.theme.current.colors;

        let active = self.active_account_entry();
        let email = active.map(|a| a.email.as_str());
        let server = active.map(|a| a.server_url.as_str()).unwrap_or("");

        let main_window_width = self
            .windows
            .get(&self.main_window_id())
            .map(|info| info.size.width)
            .unwrap_or(1024.0);

        let rctx = RenderCtx {
            colors,
            favicon: &self.favicon,
            show_favicons: self.settings.show_favicons,
            window_width: main_window_width,
            active_user: self.active_user.as_ref(),
            active_email: email,
            accounts: &self.cache.accounts,
            open_overlay: self.open_overlay,
        };

        // Inner content for authenticated screens. Vault / Send return just
        // the "right-hand" content area — the sidebar is composed below so
        // it persists across screen switches without each view re-rendering
        // it.
        let inner: Element<'_, Message, AppTheme> = match self.screen {
            Screen::Loading => {
                iced::widget::center(crate::components::spinner::spinner(48.0, colors.accent))
                    .into()
            }
            Screen::Login => self.views.login.view(server, &rctx).map(Message::login),
            Screen::Vault => self.views.vault.view(&rctx).map(Message::vault),
            Screen::Send => self.views.send.view(&rctx).map(Message::send),
        };

        let page: Element<'_, Message, AppTheme> =
            if matches!(self.screen, Screen::Vault | Screen::Send) {
                let organizations: &[crate::services::sdk::Organization] = self
                    .active_user
                    .as_ref()
                    .and_then(|uid| self.views.vault.organizations_for(uid))
                    .unwrap_or(&[]);
                let sidebar_el =
                    sidebar::view(&self.sidebar, organizations, colors).map(Message::Sidebar);
                let main_row = iced::widget::container(
                    iced::widget::row![sidebar_el, inner].height(iced::Fill),
                )
                .width(iced::Fill)
                .height(iced::Fill)
                .style(|theme: &AppTheme| {
                    iced::widget::container::Style::default().background(theme.colors.header_bg)
                });
                iced::widget::container(main_row)
                    .width(iced::Fill)
                    .height(iced::Fill)
                    .style(|theme: &AppTheme| {
                        iced::widget::container::Style::default()
                            .background(theme.colors.background)
                    })
                    .into()
            } else {
                inner
            };

        let close_toast = |idx| Message::System(SystemMessage::CloseToast(idx));

        // Bottom-sheet overlay — lives above the sidebar and title bar.
        let sheet: Option<Element<'_, Message, AppTheme>> = match self.screen {
            Screen::Vault => self
                .views
                .vault
                .sheet_view(&rctx)
                .map(|el| el.map(Message::vault)),
            Screen::Send => self
                .views
                .send
                .sheet_view(&rctx)
                .map(|el| el.map(Message::send)),
            _ => None,
        };

        // Delete-confirmation modal.
        let modal: Option<Element<'_, Message, AppTheme>> = match self.screen {
            Screen::Vault => self
                .views
                .vault
                .modal_view(&rctx)
                .map(|el| el.map(Message::vault)),
            Screen::Send => self
                .views
                .send
                .modal_view(&rctx)
                .map(|el| el.map(Message::send)),
            _ => None,
        };

        // Settings modal — composed above the vault modal so it sits on top
        // of any other overlays.
        let settings_modal: Option<Element<'_, Message, AppTheme>> = self
            .views
            .settings
            .modal_view(colors)
            .map(|el| el.map(Message::settings));

        // Generator modal — same tier as Settings. Both `modal_view` returns
        // `None` when closed so the stack stays cheap (CLAUDE.md → "Stack
        // doesn't cull or clip").
        let generator_modal: Option<Element<'_, Message, AppTheme>> = self
            .views
            .generator
            .modal_view(colors)
            .map(|el| el.map(Message::generator));

        let use_custom_menu_bar = crate::services::menu::should_use_custom_menu_bar();

        let tb: Element<'_, Message, AppTheme> = if use_custom_menu_bar {
            let menu_state = self.menu_state();
            let (open_menu, open_submenu) = match self.open_overlay {
                Some(Overlay::TitleBarMenu { menu, submenu }) => (Some(menu), submenu),
                _ => (None, None),
            };
            self.views
                .title_bar
                .view(
                    self.main_window_maximized(),
                    &menu_state,
                    open_menu,
                    open_submenu,
                    colors,
                )
                .map(Message::title_bar)
        } else {
            title_bar::TitleBarView::view_empty().map(Message::title_bar)
        };

        let main_column: Element<'_, Message, AppTheme> =
            iced::widget::column![tb, page].height(iced::Fill).into();

        // All optional layers above `main_column`, in z-order (lowest first).
        // `flatten()` drops the `None`s so the resulting Vec contains only
        // the overlays that actually need to render this frame.
        let mut overlays: Vec<Element<'_, Message, AppTheme>> =
            [sheet, modal, settings_modal, generator_modal]
                .into_iter()
                .flatten()
                .collect();

        // Drag-by-titlebar overlay sits on top of everything else when any
        // overlay is up. Only meaningful when the custom title bar is in use
        // — macOS uses the native title bar, which already handles drag.
        if use_custom_menu_bar && !overlays.is_empty() {
            let drag_strip = iced::widget::mouse_area(
                iced::widget::container(iced::widget::Space::new())
                    .width(iced::Fill)
                    .height(iced::Length::Fixed(title_bar::TITLE_BAR_HEIGHT)),
            )
            .on_press(Message::title_bar(TitleBarMessage::DragStart));
            let filler = iced::widget::container(iced::widget::Space::new())
                .width(iced::Fill)
                .height(iced::Fill);
            overlays.push(
                iced::widget::column![drag_strip, filler]
                    .width(iced::Fill)
                    .height(iced::Fill)
                    .into(),
            );
        }

        let stacked: Element<'_, Message, AppTheme> = if overlays.is_empty() {
            main_column
        } else {
            let mut layers = Vec::with_capacity(overlays.len() + 1);
            layers.push(main_column);
            layers.extend(overlays);
            iced::widget::Stack::with_children(layers)
                .width(iced::Fill)
                .height(iced::Fill)
                .into()
        };

        let with_toasts: Element<'_, Message, AppTheme> =
            toast::Manager::new(stacked, &self.toasts, close_toast).into();

        if use_custom_menu_bar {
            title_bar::resize_wrapper(with_toasts, |dir| {
                Message::title_bar(TitleBarMessage::ResizeEdge(dir))
            })
        } else {
            with_toasts
        }
    }
}
