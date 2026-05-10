//! Platform / OS integration handlers — window chrome, system events, menu
//! dispatch, tray actions. Kept together because menu/tray actions frequently
//! manipulate the window and the window-message handler dispatches menu
//! actions on keyboard shortcuts.

use iced::Task;

use crate::{
    app::{
        App, Message, SystemMessage, WindowMessage,
        window::{WindowInfo, WindowKind},
    },
    components::toast::Toast,
    domain::Screen,
    fl,
    views::{settings::SettingsSnapshot, title_bar::WindowAction},
};

impl App {
    // ── Window lifecycle ───────────────────────────────────────────────────

    pub(crate) fn handle_window_action(&mut self, action: WindowAction) -> Task<Message> {
        let id = self.main_window_id();
        match action {
            WindowAction::Minimize => {
                if self.settings.minimize_to_tray && self.ensure_tray() {
                    self.hide_main_window()
                } else {
                    iced::window::minimize(id, true)
                }
            }
            WindowAction::Maximize => {
                if let Some(info) = self.windows.get_mut(&id) {
                    info.maximized = !info.maximized;
                }
                iced::window::toggle_maximize(id)
            }
            WindowAction::Close => {
                if self.settings.close_to_tray && self.ensure_tray() {
                    self.hide_main_window()
                } else {
                    iced::window::close(id)
                }
            }
            WindowAction::Drag => iced::window::drag(id),
            WindowAction::ResizeEdge(dir) => iced::window::drag_resize(id, dir),
        }
    }

    pub(crate) fn handle_window_message(&mut self, msg: WindowMessage) -> Task<Message> {
        match msg {
            WindowMessage::Opened(id) => {
                if id == self.main_window_id() {
                    iced::window::raw_id::<Message>(id)
                        .map(move |raw| Message::Window(WindowMessage::GotRawId(id, raw)))
                } else {
                    Task::none()
                }
            }
            WindowMessage::GotRawId(_id, raw_id) => {
                self.native_menu = crate::services::menu::attach_menu(raw_id);
                Task::none()
            }
            WindowMessage::CloseRequested(id) => {
                // Route OS-native close to the same handler as the custom
                // title-bar X so close-to-tray applies uniformly. Other
                // windows (About) close normally.
                if id == self.main_window_id() {
                    self.handle_window_action(WindowAction::Close)
                } else {
                    iced::window::close(id)
                }
            }
            WindowMessage::Closed(id) => {
                let was_main = id == self.main_window_id();
                self.windows.remove(&id);
                if was_main { iced::exit() } else { Task::none() }
            }
            WindowMessage::Resized(id, size) => {
                if let Some(info) = self.windows.get_mut(&id) {
                    info.size = size;
                }
                Task::none()
            }
            WindowMessage::Focused(id) => {
                if id == self.main_window_id() {
                    self.main_window_focused = true;
                    // The active user just became eligible for the grace
                    // exemption from `lock_after`; recompute drops their
                    // `lock_after` deadline from the watch.
                    self.refresh_session_timeout_deadline();
                }
                Task::none()
            }
            WindowMessage::Unfocused(id) => {
                if id == self.magnify.window {
                    return self
                        .handle_magnify_message(crate::views::magnify::MagnifyMessage::Hide);
                }
                if id == self.main_window_id() {
                    self.main_window_focused = false;
                    self.refresh_session_timeout_deadline();
                }
                Task::none()
            }
            WindowMessage::MouseInput(id) => {
                // Only main-window clicks count toward the active user's
                // session timer; the Magnify launcher dismisses on
                // focus-loss, so its clicks shouldn't extend the timer.
                if id == self.main_window_id() {
                    self.record_session_activity();
                }
                Task::none()
            }
            WindowMessage::KeyPressed(id, ev) => {
                // Magnify gets first crack at keys for its own window.
                if id == self.magnify.window
                    && let Some(task) = self.handle_magnify_key(ev.clone())
                {
                    return task;
                }
                // Shortcuts only fire on the main window — Ctrl+F in the
                // About window must not trigger vault search.
                if id != self.main_window_id() {
                    return Task::none();
                }
                // Recorded before the shortcut dispatch so e.g. Ctrl+L
                // (lock vault) pushes back any logout deadline before the
                // lock takes effect.
                self.record_session_activity();
                let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = ev else {
                    return Task::none();
                };
                // Escape closes the settings modal before any menu shortcut
                // lookup, otherwise it would fall through to widgets behind
                // the modal.
                if matches!(
                    key,
                    iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape)
                ) && self.views.settings.is_open()
                {
                    self.views.settings.close();
                    return Task::none();
                }
                let state = self.menu_state();
                let Some(action) =
                    crate::services::menu::find_shortcut_action(&key, modifiers, &state)
                else {
                    return Task::none();
                };
                self.open_overlay = None;
                self.handle_menu_action(action)
            }
        }
    }

    // ── System signals ─────────────────────────────────────────────────────

    pub(crate) fn handle_system_message(&mut self, msg: SystemMessage) -> Task<Message> {
        match msg {
            SystemMessage::MudaEvent(event) => {
                // Native menu matches win; tray menu is the fallback (both
                // share one global muda receiver).
                if let Some(handle) = &self.native_menu
                    && let Some(action) = handle.resolve(&event.id)
                {
                    return self.handle_menu_action(action);
                }
                if let Some(handle) = &self.tray
                    && let Some(action) = handle.resolve(&event.id)
                {
                    return self.handle_tray_action(action);
                }
                Task::none()
            }
            SystemMessage::TrayClick(action) => self.handle_tray_action(action),
            SystemMessage::ThemeChanged => {
                self.theme.refresh();
                Task::none()
            }
            SystemMessage::CloseToast(idx) => {
                if idx < self.toasts.len() {
                    self.toasts.remove(idx);
                }
                Task::none()
            }
            SystemMessage::ClientManagerLoaded(slot) => {
                // Recover the inner Option even if the load future panicked
                // before sending — the value (if any) is still valid.
                let Some(mgr) = slot.lock().unwrap_or_else(|e| e.into_inner()).take() else {
                    tracing::error!("ClientManager slot was already drained");
                    return Task::none();
                };
                self.client_manager = mgr;
                self.active_user = self.client_manager.user_ids().into_iter().next();
                self.show_login_for_active()
            }
            SystemMessage::InstanceWakeRequested => self.show_main_window(),
            SystemMessage::SessionEvent(ev) => {
                use session_events::SessionEvent;
                match ev {
                    SessionEvent::Suspended => {
                        // Snapshot wall + monotonic so we can compute the
                        // suspend gap on resume. Users with the
                        // `lock_on_system_lock` preference are locked
                        // immediately; the rest stay enrolled so their
                        // `last_activity` can be back-dated on `Resumed`.
                        crate::services::session_timeout::note_suspend();
                        self.lock_system_lock_users()
                    }
                    SessionEvent::Locked => self.lock_system_lock_users(),
                    SessionEvent::Resumed => {
                        // Reconcile last_activity for the time CLOCK_MONOTONIC
                        // missed during suspend, then re-check timeouts.
                        crate::services::session_timeout::note_resume();
                        self.run_session_timeout_check()
                    }
                    SessionEvent::Unlocked => Task::none(),
                }
            }
            SystemMessage::SessionTimeoutCheck => self.run_session_timeout_check(),
            SystemMessage::SyncCompleted(res) => {
                match res {
                    Ok(()) => self.push_toast(Toast::success(fl!("menu-toast-sync-success"), None)),
                    Err(err) => {
                        tracing::warn!(%err, "vault sync failed");
                        self.push_toast(Toast::error(
                            fl!("menu-toast-sync-failed-body"),
                            Some(&fl!("menu-toast-sync-failed-title")),
                        ));
                    }
                }
                Task::none()
            }
            #[cfg(feature = "gpu")]
            SystemMessage::WgpuBackendDiscovered(backend) => {
                // We've reached first paint, so whatever wgpu picked is
                // known-good for this hardware/driver combo. Persist for
                // next launch and clear the crash-sentinel.
                //
                // The backend string comes from `format!("{:?}", wgpu::Backend)`
                // inside iced's wgpu compositor. PascalCase → lowercase here,
                // then fed back to `WGPU_BACKEND` whose parser
                // (`Backends::from_comma_list`) accepts only the four desktop
                // names. Pinning that contract explicitly so we don't silently
                // poison settings.json if a future iced/wgpu rev:
                //   - renames a Debug variant (e.g. `Dx12` → `DirectX12`)
                //   - exposes `Noop` or `BrowserWebGpu` as a string we'd
                //     otherwise persist (Noop especially is a real env value
                //     that wgpu honours, locking us into the dummy backend)
                //   - falls back to tiny-skia, whose compositor reports
                //     `backend: "tiny-skia"`.
                let normalised = backend.to_lowercase();
                if matches!(normalised.as_str(), "vulkan" | "dx12" | "metal" | "gl") {
                    let mut s = crate::services::settings::Settings::load();
                    s.wgpu_backend_pending = None;
                    s.wgpu_backend_verified = Some(normalised);
                    s.save();
                }
                Task::none()
            }
        }
    }

    // ── Menu + tray dispatch ───────────────────────────────────────────────

    pub(crate) fn handle_menu_action(
        &mut self,
        action: crate::services::menu::MenuAction,
    ) -> Task<Message> {
        use crate::services::menu::MenuAction;
        match action {
            MenuAction::Quit => {
                return iced::exit();
            }
            MenuAction::LockAllVaults => {
                let uids = self.client_manager.user_ids();
                let tasks: Vec<_> = uids.iter().map(|uid| self.lock_user(uid)).collect();
                return Task::batch(tasks);
            }
            MenuAction::ToggleFullScreen => {
                let id = self.main_window_id();
                if let Some(info) = self.windows.get_mut(&id) {
                    info.fullscreen = !info.fullscreen;
                    let mode = if info.fullscreen {
                        iced::window::Mode::Fullscreen
                    } else {
                        iced::window::Mode::Windowed
                    };
                    return iced::window::set_mode(id, mode);
                }
            }
            MenuAction::Minimize => {
                return iced::window::minimize(self.main_window_id(), true);
            }
            MenuAction::Close => {
                return iced::window::close(self.main_window_id());
            }
            MenuAction::SearchVault => {
                if self.screen == Screen::Vault {
                    return self.views.vault.focus_search_task().map(Message::vault);
                }
            }
            MenuAction::SyncNow => {
                let Some(uid) = self.active_user else {
                    return Task::none();
                };
                if self.client_manager.client_for(&uid).is_none() {
                    return Task::none();
                }
                return Task::perform(crate::services::sdk::sync(), |res| {
                    Message::System(SystemMessage::SyncCompleted(res.map_err(|e| e.to_string())))
                });
            }
            MenuAction::HideToTray => {
                if self.ensure_tray() {
                    return self.hide_main_window();
                }
            }
            MenuAction::ToggleAlwaysOnTop => {
                let id = self.main_window_id();
                if let Some(info) = self.windows.get_mut(&id) {
                    info.always_on_top = !info.always_on_top;
                    let level = if info.always_on_top {
                        iced::window::Level::AlwaysOnTop
                    } else {
                        iced::window::Level::Normal
                    };
                    return iced::window::set_level(id, level);
                }
            }
            MenuAction::Settings => {
                if let Some(uid) = self.active_user {
                    self.open_overlay = None;

                    let snap = SettingsSnapshot {
                        settings: self.settings.clone(),
                        prefs: self.settings.preferences_for(&uid),
                    };
                    self.views.settings.open_with(snap);
                }
            }
            MenuAction::Generator => {
                return self.open_generator_modal();
            }
            MenuAction::GeneratorHistory => {
                return self.open_generator_history();
            }
            MenuAction::ZoomIn => {
                if self.settings.zoom_factor.step_in() {
                    self.settings.save();
                }
            }
            MenuAction::ZoomOut => {
                if self.settings.zoom_factor.step_out() {
                    self.settings.save();
                }
            }
            MenuAction::ZoomReset => {
                if self.settings.zoom_factor.reset() {
                    self.settings.save();
                }
            }
            MenuAction::ToggleHardwareAcceleration => {
                // The renderer backend is selected once at startup
                // (`main::select_backend`), so flipping the bit here only
                // takes effect on next launch. Persist + toast so the user
                // knows a restart is needed.
                self.settings.hardware_acceleration = !self.settings.hardware_acceleration;
                self.settings.save();
                let toast_msg = if self.settings.hardware_acceleration {
                    crate::fl!("menu-help-toast-hw-accel-on")
                } else {
                    crate::fl!("menu-help-toast-hw-accel-off")
                };
                self.push_toast(crate::components::toast::Toast::info(toast_msg, None));
            }
            MenuAction::Import => {
                return self.open_import_modal();
            }
            MenuAction::Export => {
                return self.open_export_modal();
            }
            MenuAction::NewFolder => {
                return self.open_new_folder_modal();
            }
            MenuAction::NewItem(t) => {
                if self.screen != Screen::Vault {
                    self.set_screen(Screen::Vault);
                }
                self.open_overlay = None;
                return Task::done(Message::vault(crate::views::vault::VaultMessage::NewItem(
                    t,
                )));
            }
            MenuAction::CopyUsername => {
                return self.copy_from_active_selection(
                    crate::views::vault::widgets::cipher_detail::CipherDetailMessage::CopyUsername,
                );
            }
            MenuAction::CopyPassword => {
                return self.copy_from_active_selection(
                    crate::views::vault::widgets::cipher_detail::CipherDetailMessage::CopyPassword,
                );
            }
            MenuAction::CopyTotp => {
                return self.copy_from_active_selection(
                    crate::views::vault::widgets::cipher_detail::CipherDetailMessage::CopyTotp,
                );
            }
            MenuAction::FingerprintPhrase => {
                let Some(uid) = self.active_user else {
                    return Task::none();
                };
                let Some(phrase) = self.client_manager.user_fingerprint(&uid) else {
                    return Task::none();
                };
                self.open_overlay = None;
                self.fingerprint.open_with(phrase);
            }
            MenuAction::OpenStaticUrl(url) => {
                crate::services::clipboard::launch_url(url);
            }
            MenuAction::OpenWebVault(path) => {
                let Some(base) = self
                    .active_account_entry()
                    .map(|a| a.server_url.as_str())
                    .filter(|s| !s.is_empty())
                else {
                    return Task::none();
                };
                let url = match path {
                    Some(p) => format!("{}/{}", base.trim_end_matches('/'), p),
                    None => base.to_string(),
                };
                crate::services::clipboard::launch_url(&url);
            }
            MenuAction::About => {
                if let Some(id) = self.about_window_id() {
                    return iced::window::gain_focus(id);
                }

                let about_size = iced::Size::new(400.0, 280.0);
                let (about_id, open_task) = iced::window::open(iced::window::Settings {
                    size: about_size,
                    min_size: Some(iced::Size::new(360.0, 260.0)),
                    position: iced::window::Position::Centered,
                    resizable: false,
                    decorations: true,
                    icon: iced::window::icon::from_file_data(
                        crate::assets::ICON_PNG,
                        Some(image::ImageFormat::Png),
                    )
                    .ok(),
                    ..Default::default()
                });

                self.windows
                    .insert(about_id, WindowInfo::new(WindowKind::About, about_size));

                return open_task.map(|id| Message::Window(WindowMessage::Opened(id)));
            }
        }
        Task::none()
    }

    fn handle_tray_action(&mut self, action: crate::services::tray::TrayAction) -> Task<Message> {
        use crate::services::tray::TrayAction;
        match action {
            TrayAction::ToggleShowHide => self.toggle_main_window_visibility(),
            TrayAction::LockVault => {
                self.handle_menu_action(crate::services::menu::MenuAction::LockAllVaults)
            }
            TrayAction::Exit => iced::exit(),
        }
    }

    // ── Window visibility helpers (tray + IPC wake) ────────────────────────

    fn hide_main_window(&mut self) -> Task<Message> {
        iced::window::set_mode(self.main_window_id(), iced::window::Mode::Hidden)
    }

    fn show_main_window(&mut self) -> Task<Message> {
        let id = self.main_window_id();
        Task::batch([
            iced::window::set_mode(id, iced::window::Mode::Windowed),
            iced::window::gain_focus(id),
        ])
    }

    /// Ask iced for the current mode and flip it. `Task::then` defers the
    /// decision until the mode resolves — one round-trip slower than reading
    /// a local bool, but iced stays the single source of truth so we can't
    /// drift from the OS-level window state. `Fullscreen` counts as visible.
    fn toggle_main_window_visibility(&mut self) -> Task<Message> {
        let id = self.main_window_id();
        iced::window::mode(id).then(move |mode| match mode {
            iced::window::Mode::Hidden => Task::batch([
                iced::window::set_mode(id, iced::window::Mode::Windowed),
                iced::window::gain_focus(id),
            ]),
            iced::window::Mode::Windowed | iced::window::Mode::Fullscreen => {
                iced::window::set_mode(id, iced::window::Mode::Hidden)
            }
        })
    }

    /// Lazily build the tray. Callers hiding the window to the tray MUST
    /// check the return: on `false`, fall through to a normal minimize/close,
    /// otherwise the window becomes unrecoverable (hidden, no tray icon).
    fn ensure_tray(&mut self) -> bool {
        if self.tray.is_none() {
            self.tray = crate::services::tray::build();
            if self.tray.is_none() {
                tracing::warn!("tray requested but failed to initialise; continuing without tray");
            }
        }
        self.tray.is_some()
    }
}
