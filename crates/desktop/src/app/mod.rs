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
        export as export_view, generator as generator_view, import as import_view, login, magnify,
        new_folder as new_folder_view, send, settings as settings_view,
        title_bar::{self, TitleBarMessage},
        vault,
    },
};

use helpers::main_window_platform_specific;

pub struct App {
    // ── Session ───────────────────────────────────────────────────────────
    pub(super) active_user: Option<UserId>,
    pub(super) client_manager: Arc<ClientManager>,
    /// Read once at startup. Lifecycle branches re-read `self.settings.<field>`
    /// at event time (never cached) so live changes apply without a restart.
    pub(super) settings: Settings,

    // ── Navigation ────────────────────────────────────────────────────────
    pub(super) screen: Screen,
    /// Persists across screen transitions so collapse / filter selection
    /// isn't reset when navigating between Vault and Send.
    pub(super) sidebar: SidebarState,

    // ── Sub-views ─────────────────────────────────────────────────────────
    pub(super) views: Views,

    // ── Windows ───────────────────────────────────────────────────────────
    pub(super) windows: HashMap<iced::window::Id, WindowInfo>,
    /// Always present from `App::new` until `iced::exit`. Child windows
    /// (About) are not tracked here.
    pub(super) main_window: iced::window::Id,
    pub(super) magnify: magnify::MagnifyView,

    // ── Native chrome ─────────────────────────────────────────────────────
    pub(super) theme: ThemeState,
    pub(super) native_menu: Option<crate::services::menu::NativeMenuHandle>,
    pub(super) tray: Option<TrayHandle>,

    // ── Cross-cutting services ────────────────────────────────────────────
    /// Single writer: every clipboard `set` flows through here so the 30 s
    /// auto-clear bookkeeping sees every write.
    pub(super) clipboard: ClipboardManager,
    pub(super) favicon: crate::services::favicon::FaviconService,

    // ── Transient UI ──────────────────────────────────────────────────────
    /// Source of truth for which dropdown/menu is open. Writing auto-
    /// dismisses any other overlay by construction.
    pub(super) open_overlay: Option<Overlay>,
    pub(super) toasts: Vec<Toast>,
    /// Account → Fingerprint phrase modal. Closed unless the user explicitly
    /// opened it via the menu.
    pub(super) fingerprint: crate::views::fingerprint_phrase::FingerprintModal,

    // ── Derived ───────────────────────────────────────────────────────────
    pub(super) cache: ViewCache,
}

/// Derived data cached across `view()` rebuilds. Refreshed only by handlers
/// that mutate the inputs (login, logout, lock, unlock, user switch, manager
/// load). Cached on App because `view()` returns an `Element<'_, ...>`
/// borrowing `&self` — slices handed to children can't be built inline.
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
    pub(super) import: import_view::ImportView,
    pub(super) export: export_view::ExportView,
    pub(super) new_folder: new_folder_view::NewFolderView,
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
            import: import_view::ImportView::new(),
            export: export_view::ExportView::new(),
            new_folder: new_folder_view::NewFolderView::new(),
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
        // (already handled by `i18n::init()` in `main`).
        if !settings.language.is_empty()
            && let Ok(tag) = settings.language.parse()
        {
            crate::services::i18n::set_language(tag);
        }

        // Must run *before* any menu or tray icon is built — both crates'
        // `set_event_handler` are one-shot.
        crate::services::menu::install_event_handler();
        crate::services::tray::install_event_handler();
        // Global hotkey registration failure (e.g. Wayland) is logged
        // internally; Magnify is then just unreachable.
        crate::services::global_hotkey::install_event_handler();

        // `--autostart` forces the app to launch hidden in the tray. Force
        // the tray to build in that case even if no tray settings are on,
        // otherwise the hidden window would have no entry point.
        let autostart = std::env::args().any(|a| a == "--autostart");

        let mut tray = None;
        if settings.wants_tray() || autostart {
            tray = crate::services::tray::build();
            if tray.is_none() {
                tracing::warn!("tray requested but failed to initialise");
            }
        }

        // When `--autostart` is set and the tray initialised, open hidden so
        // the user sees only the tray. `visible: false` flows through winit's
        // `with_visible(false)` — no flash, iced still owns the id and keeps
        // firing `view()`. Toggling later is a cheap `Mode::Windowed`.
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

        // Pre-create the magnify launcher hidden so subsequent hotkey presses
        // are a cheap show/hide instead of paying for window creation +
        // first-paint mid-summon. The wgpu device is shared with the main
        // window so we don't repay adapter/driver init.
        let (magnify_id, magnify_size, magnify_open_task) =
            handlers::magnify::open_magnify_window();
        windows.insert(
            magnify_id,
            WindowInfo::new(WindowKind::Magnify, magnify_size),
        );

        // Discover users in `<workspace-root>/data/` and open one SQLite DB
        // per user. The `ClientManagerLoaded` handler swaps the Arc when done.
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
            client_manager: Arc::new(ClientManager::empty()),
            settings,

            screen: Screen::Loading,
            sidebar: SidebarState::default(),

            views: Views::new(),

            windows,
            main_window: main_id,
            magnify: magnify::MagnifyView::new(magnify_id),

            theme: ThemeState::new(user_theme),
            native_menu: None,
            tray,

            clipboard: ClipboardManager::new(),
            favicon,

            open_overlay: None,
            toasts: Vec::new(),
            fingerprint: crate::views::fingerprint_phrase::FingerprintModal::default(),

            cache: ViewCache::default(),
        };

        // Resolves only after the wgpu compositor exists (so the backend wgpu
        // ended up using is final). Drives `Settings::wgpu_backend_verified`
        // for the next-launch fast path. Compiled to `Task::none()` when the
        // `gpu` feature is off so the discovery plumbing only exists in builds
        // that can actually use a wgpu backend.
        #[cfg(feature = "gpu")]
        let info_task = iced::system::information().map(|info| {
            Message::System(SystemMessage::WgpuBackendDiscovered(info.graphics_backend))
        });
        #[cfg(not(feature = "gpu"))]
        let info_task = Task::none();

        (
            app,
            Task::batch([
                open_task.map(|id| Message::Window(WindowMessage::Opened(id))),
                magnify_open_task,
                load_task,
                info_task,
            ]),
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let close_sub =
            iced::window::close_events().map(|id| Message::Window(WindowMessage::Closed(id)));

        // Routed through `event::listen_with` (not `keyboard::listen`) so the
        // `window::Id` is provided natively. `Subscription::map` requires a
        // non-capturing closure, and we need the id bound into the emitted
        // message for daemon multi-window dispatch.
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

        // muda's `MenuEvent::receiver()` is global — both the native app menu
        // and the tray context menu dispatch to it. The handler filters by
        // `MenuId` to decide which one fired.
        let muda_sub = if self.native_menu.is_some() || self.tray.is_some() {
            Subscription::run(crate::services::menu::muda_event_stream)
                .map(|ev| Message::System(SystemMessage::MudaEvent(ev)))
        } else {
            Subscription::none()
        };

        let tray_sub = if self.tray.is_some() {
            Subscription::run(crate::services::tray::click_stream)
                .map(|action| Message::System(SystemMessage::TrayClick(action)))
        } else {
            Subscription::none()
        };

        let theme_sub = Subscription::run_with(self.theme.system.clone(), |st| st.subscribe())
            .map(|_| Message::System(SystemMessage::ThemeChanged));

        // Second-launch wake-up: listener bound once inside this stream, kept
        // alive across `update()` cycles because iced hashes the subscription
        // identity from the `fn` pointer.
        let wake_sub = Subscription::run(crate::services::instance_lock::wake_stream)
            .map(|_| Message::System(SystemMessage::InstanceWakeRequested));

        // OS session state (screen lock, suspend). Bound once; the platform
        // module owns the OS-side observer registration.
        let session_sub = Subscription::run(session_events::event_stream)
            .map(|ev| Message::System(SystemMessage::SessionEvent(ev)));

        // Idle when OS-side hotkey registration failed (Wayland, missing
        // permissions) — the stream terminates and the subscription stays
        // dormant.
        let magnify_sub = Subscription::run(crate::services::global_hotkey::event_stream)
            .map(|_| Message::Magnify(crate::views::magnify::MagnifyMessage::HotkeyPressed));

        // Favicon fetch completions. Each message flips one row from globe
        // to real icon by virtue of arriving.
        let favicon_sub =
            Subscription::run(crate::services::favicon::favicon_event_stream).map(Message::Favicon);

        // Only subscribed while at least one transition might still be
        // running. `any_in_progress` is a single global-watermark read, so
        // animation primitives that call `animation::extend(...)`
        // auto-register here without extra wiring.
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
            session_sub,
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
            // No-op — the redraw the message triggers is the entire point.
            // Subscription rebuilds and unsubscribes once nothing's animating.
            Message::AnimationTick => Task::none(),
            Message::Favicon(crate::services::favicon::FaviconMessage::IconResolved {
                uid,
                hostname,
            }) => {
                // Arrival drives the redraw; the service has already mutated
                // its in-memory cache. Log at trace so a cold unlock with
                // thousands of icons doesn't spam RUST_LOG=info users.
                tracing::trace!(%uid, %hostname, "favicon resolved");
                Task::none()
            }
            Message::View(view_msg) => {
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
                    ViewMessage::Import(m) => self
                        .views
                        .import
                        .update(m, uctx)
                        .dispatch(Message::import, |e| self.handle_import_event(e)),
                    ViewMessage::Export(m) => self
                        .views
                        .export
                        .update(m, uctx)
                        .dispatch(Message::export, |e| self.handle_export_event(e)),
                    ViewMessage::NewFolder(m) => self
                        .views
                        .new_folder
                        .update(m, uctx)
                        .dispatch(Message::new_folder, |e| self.handle_new_folder_event(e)),
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
            // Defensive: all windows are inserted at creation, but a stray
            // unknown id renders as empty space rather than panicking.
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
        // Magnify launcher needs a transparent OS-window background so pixels
        // outside its rounded container stay see-through.
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

        // Vault / Send return just the right-hand content area — the sidebar
        // is composed below so it persists across screen switches without
        // each view re-rendering it.
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

        let sheet = match self.screen {
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

        let modal = match self.screen {
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
            Screen::Login => self
                .views
                .login
                .modal_view(colors)
                .map(|el| el.map(Message::login)),
            _ => None,
        };

        let settings_modal = self
            .views
            .settings
            .modal_view(colors)
            .map(|el| el.map(Message::settings));

        // Both `modal_view` returns `None` when closed so the stack stays
        // cheap (CLAUDE.md → "Stack doesn't cull or clip").
        let generator_modal = self
            .views
            .generator
            .modal_view(colors)
            .map(|el| el.map(Message::generator));

        let import_modal = self
            .views
            .import
            .modal_view(colors)
            .map(|el| el.map(Message::import));

        let export_modal = self
            .views
            .export
            .modal_view(colors)
            .map(|el| el.map(Message::export));

        let new_folder_modal = self
            .views
            .new_folder
            .modal_view(colors)
            .map(|el| el.map(Message::new_folder));

        let fingerprint_modal =
            crate::views::fingerprint_phrase::modal_view(&self.fingerprint, colors);

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

        // Optional layers above `main_column`, in z-order (lowest first).
        // `flatten()` drops the `None`s so only overlays that need to render
        // this frame end up in the Vec.
        let mut overlays: Vec<Element<'_, Message, AppTheme>> = [
            sheet,
            modal,
            settings_modal,
            generator_modal,
            import_modal,
            export_modal,
            new_folder_modal,
            fingerprint_modal,
        ]
        .into_iter()
        .flatten()
        .collect();

        // Drag-by-titlebar overlay sits on top when any overlay is up. Only
        // meaningful with the custom title bar — macOS native already handles
        // drag.
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
