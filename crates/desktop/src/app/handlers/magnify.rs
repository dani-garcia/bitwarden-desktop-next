//! Magnify launcher dispatch — hotkey handling, summon / hide, async copy.
//!
//! Magnify lives in its own window so it can't ride the screen-driven
//! `ViewMessage` routing the other views use. Instead, `Message::Magnify`
//! arrives directly here and we mutate `App::magnify` plus return a
//! `Task<Message>` for any window / clipboard / decrypt work.

use std::time::Instant;

use iced::Task;

use crate::{
    app::{App, Message},
    domain::Screen,
    services::{clipboard::Sensitivity, cursor_monitor},
    views::magnify::{MAGNIFY_RESULTS_SCROLL_ID, MAGNIFY_SEARCH_ID, MagnifyMessage, Mode, dims},
};

impl App {
    pub(crate) fn handle_magnify_message(&mut self, msg: MagnifyMessage) -> Task<Message> {
        match msg {
            MagnifyMessage::HotkeyPressed => self.magnify_hotkey(),
            MagnifyMessage::WindowOpened(_id) => {
                // Window is created hidden at startup in `App::new`. The first
                // hotkey press goes through `magnify_hotkey`'s toggle path,
                // which handles focus / select-all / scroll when the window
                // actually becomes visible — nothing to do here.
                Task::none()
            }
            MagnifyMessage::QueryChanged(query) => {
                self.magnify.query = query;
                // Fresh search — reset selection + scroll because the
                // result set is about to change. `recompute` only clamps;
                // it preserves selection for sticky-restore summons.
                self.magnify.selected = 0;
                self.magnify.scroll_offset_y = 0.0;
                self.magnify_recompute();
                Task::batch([
                    self.magnify_resize_task(),
                    iced::widget::operation::scroll_to(
                        MAGNIFY_RESULTS_SCROLL_ID,
                        iced::widget::scrollable::AbsoluteOffset {
                            x: None,
                            y: Some(0.0),
                        },
                    ),
                ])
            }
            MagnifyMessage::NavigateUp => {
                if self.magnify.selected > 0 {
                    self.magnify.selected -= 1;
                }
                self.magnify_keep_selected_visible()
            }
            MagnifyMessage::NavigateDown => {
                let len = self.magnify.results.len();
                if len > 0 && self.magnify.selected + 1 < len {
                    self.magnify.selected += 1;
                }
                self.magnify_keep_selected_visible()
            }
            MagnifyMessage::Scrolled(viewport) => {
                self.magnify.scroll_offset_y = viewport.absolute_offset().y;
                Task::none()
            }
            MagnifyMessage::RowClicked(idx) => {
                if idx < self.magnify.results.len() {
                    self.magnify.selected = idx;
                }
                Task::none()
            }
            MagnifyMessage::CopyPasswordRequested => self.magnify_copy_password(),
            MagnifyMessage::PasswordDecryptCompleted(uid, cipher_id, result) => {
                // Stale completion: cipher id doesn't match the one we
                // requested, OR the active user changed mid-decrypt, OR the
                // user is no longer unlocked (locked between request and
                // completion — must NOT copy a freshly-decrypted secret to
                // a now-locked session). Crucially, do NOT clear
                // `pending_password` here: a *different* in-flight decrypt's
                // completion would clobber a still-valid pending request.
                if self.magnify.pending_password != Some(cipher_id)
                    || self.active_user != Some(uid)
                    || !self.client_manager.is_unlocked(&uid)
                {
                    return Task::none();
                }
                self.magnify.pending_password = None;
                match result {
                    Ok(Some(password)) => {
                        self.clipboard.copy(password, Sensitivity::Sensitive);
                    }
                    Ok(None) => {
                        tracing::debug!(%cipher_id, "magnify: cipher has no password field");
                    }
                    Err(e) => {
                        // No toast: the main window is likely hidden (the
                        // launcher dismissed itself when the user pressed
                        // Ctrl+C), so a toast there would never be seen.
                        // Tray-level error reporting is a separate
                        // problem; for now the warning is enough.
                        tracing::warn!(error = %e, %cipher_id, "magnify: password decrypt failed");
                    }
                }
                Task::none()
            }
            MagnifyMessage::CopyUsernameRequested => {
                if let Some(item) = self.magnify.selected_item() {
                    let subtitle = item.subtitle.as_str();
                    if !subtitle.is_empty() {
                        self.clipboard
                            .copy(subtitle.to_string(), Sensitivity::Normal);
                    }
                }
                self.magnify_hide()
            }
            MagnifyMessage::Hide => self.magnify_hide(),
            MagnifyMessage::OpenMainWindow => self.magnify_open_main(),
        }
    }

    /// Routes a key press inside the magnify window to the appropriate
    /// `MagnifyMessage`. Called from the central `WindowMessage::KeyPressed`
    /// handler when `id == magnify.window`.
    pub(crate) fn handle_magnify_key(
        &mut self,
        ev: iced::keyboard::Event,
    ) -> Option<Task<Message>> {
        use iced::keyboard::{Event::KeyPressed, Key, key::Named};

        let KeyPressed { key, modifiers, .. } = ev else {
            return None;
        };

        let msg = match (&key, modifiers.command(), modifiers.shift()) {
            (Key::Named(Named::Escape), _, _) => MagnifyMessage::Hide,
            // Enter triggers the locked-state action (Open Bitwarden →
            // unlock screen). In `Mode::Unlocked` Enter is reserved for the
            // future autotype hand-off and stays a no-op here.
            (Key::Named(Named::Enter), _, _) if matches!(self.magnify.mode, Mode::Locked) => {
                MagnifyMessage::OpenMainWindow
            }
            (Key::Named(Named::ArrowUp), _, _) => MagnifyMessage::NavigateUp,
            (Key::Named(Named::ArrowDown), _, _) => MagnifyMessage::NavigateDown,
            (Key::Character(s), true, true)
                if s.chars()
                    .next()
                    .is_some_and(|c| c.eq_ignore_ascii_case(&'c')) =>
            {
                MagnifyMessage::CopyUsernameRequested
            }
            (Key::Character(s), true, false)
                if s.chars()
                    .next()
                    .is_some_and(|c| c.eq_ignore_ascii_case(&'c')) =>
            {
                MagnifyMessage::CopyPasswordRequested
            }
            _ => return None,
        };
        Some(self.handle_magnify_message(msg))
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn magnify_hotkey(&mut self) -> Task<Message> {
        let now = Instant::now();
        let active = self.active_user;
        let unlocked = active.is_some_and(|uid| self.client_manager.is_unlocked(&uid));
        self.magnify.mode = if unlocked {
            Mode::Unlocked
        } else {
            Mode::Locked
        };

        // Reset sticky state when the user changed, the active user is locked,
        // or the 5-minute TTL expired. Otherwise restore the previous query
        // and selection.
        let user_changed = self.magnify.anchored_user != active;
        let stale = !self.magnify.sticky_state_fresh(now);
        if user_changed || !unlocked || stale {
            self.magnify.reset_search();
        }
        self.magnify.anchored_user = active;

        // Refresh results against the latest item cache (covers items added
        // since the last summon AND first-time recompute when sticky state
        // was preserved).
        self.magnify_recompute();

        // Window is created at startup in `App::new`, so id is always set.
        let Some(id) = self.magnify.window else {
            tracing::error!("magnify: hotkey fired before window was created");
            return Task::none();
        };
        // Resize to match the current results count *before* the OS
        // window becomes visible — avoids a one-frame flash at the old
        // height before the resize lands.
        let height = dims::height_for(self.magnify.results.len());
        let resize = iced::window::resize::<Message>(id, iced::Size::new(dims::WIDTH, height));
        // Reposition onto the cursor's monitor so the launcher follows
        // the user across multi-monitor setups. Falls back to no-op on
        // platforms where the lookup isn't implemented; the window
        // stays where it last was.
        let reposition = magnify_reposition_for_cursor(id, height);
        // Scroll the selected row into view. Safe to fire regardless
        // of show/hide — operating on a hidden scrollable just
        // updates its offset for the next show. `Task` is not `Clone`
        // so this lives outside the `mode` continuation.
        let scroll = self.magnify_keep_selected_visible();
        Task::batch([
            resize,
            reposition,
            scroll,
            iced::window::mode(id).then(move |mode| match mode {
                iced::window::Mode::Hidden => Task::batch([
                    iced::window::set_mode(id, iced::window::Mode::Windowed),
                    iced::window::gain_focus(id),
                    iced::widget::operation::focus(MAGNIFY_SEARCH_ID).map(Message::Magnify),
                    iced::widget::operation::select_all(MAGNIFY_SEARCH_ID).map(Message::Magnify),
                ]),
                iced::window::Mode::Windowed | iced::window::Mode::Fullscreen => {
                    iced::window::set_mode(id, iced::window::Mode::Hidden)
                }
            }),
        ])
    }

    fn magnify_hide(&mut self) -> Task<Message> {
        self.magnify.touch();
        if let Some(id) = self.magnify.window {
            iced::window::set_mode(id, iced::window::Mode::Hidden)
        } else {
            Task::none()
        }
    }

    fn magnify_open_main(&mut self) -> Task<Message> {
        let hide = self.magnify_hide();
        let main_id = self.main_window_id();
        let show = Task::batch([
            iced::window::set_mode(main_id, iced::window::Mode::Windowed),
            iced::window::gain_focus(main_id),
        ]);
        self.views
            .login
            .show_unlock_for(self.active_user.as_ref(), &self.client_manager);
        self.set_screen(Screen::Login);
        Task::batch([hide, show])
    }

    fn magnify_copy_password(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        let Some(cipher_id) = self.magnify.selected_item().and_then(|item| item.id) else {
            return Task::none();
        };
        self.magnify.pending_password = Some(cipher_id);
        let mgr = self.client_manager.clone();
        let decrypt = Task::perform(
            async move {
                let result = mgr
                    .full_cipher(&uid, cipher_id)
                    .await
                    .map(|view| match view.r#type {
                        bitwarden_vault::CipherType::Login => {
                            view.login.and_then(|l| l.password).map(|p| p.to_string())
                        }
                        _ => None,
                    });
                (uid, cipher_id, result)
            },
            |(uid, cipher_id, result)| {
                Message::Magnify(MagnifyMessage::PasswordDecryptCompleted(
                    uid, cipher_id, result,
                ))
            },
        );
        Task::batch([decrypt, self.magnify_hide()])
    }

    fn magnify_recompute(&mut self) {
        let items = self
            .active_user
            .as_ref()
            .and_then(|uid| self.views.vault.all_items_for(uid));
        match items {
            Some(items) => self.magnify.recompute(items),
            None => {
                self.magnify.results.clear();
                self.magnify.selected = 0;
                self.magnify.scroll_offset_y = 0.0;
            }
        }
    }

    fn magnify_resize_task(&self) -> Task<Message> {
        let Some(id) = self.magnify.window else {
            return Task::none();
        };
        let height = dims::height_for(self.magnify.results.len());
        iced::window::resize(id, iced::Size::new(dims::WIDTH, height))
    }

    /// Scroll the results list so the selected row is at least partially
    /// visible. Returns a `scroll_to` Task only if the selected row would
    /// otherwise sit above or below the viewport — minimal scroll, no
    /// jumping when the user navigates within the visible window. Used
    /// both on arrow navigation and on summon (callers reset
    /// `scroll_offset_y` to 0 first when the scrollable was freshly
    /// mounted).
    fn magnify_keep_selected_visible(&mut self) -> Task<Message> {
        if self.magnify.results.is_empty() {
            return Task::none();
        }
        let row_h = dims::ROW_HEIGHT;
        let viewport_h = dims::MAX_VISIBLE_ROWS as f32 * row_h;
        let selected_top = self.magnify.selected as f32 * row_h;
        let selected_bottom = selected_top + row_h;
        let visible_top = self.magnify.scroll_offset_y;
        let visible_bottom = visible_top + viewport_h;

        let new_offset_y = if selected_top < visible_top {
            Some(selected_top)
        } else if selected_bottom > visible_bottom {
            Some(selected_bottom - viewport_h)
        } else {
            None
        };

        match new_offset_y {
            Some(y) => {
                // Update the cached offset eagerly so a second arrow press
                // before the `on_scroll` callback round-trips makes the
                // right decision.
                self.magnify.scroll_offset_y = y;
                iced::widget::operation::scroll_to(
                    MAGNIFY_RESULTS_SCROLL_ID,
                    iced::widget::scrollable::AbsoluteOffset {
                        x: None,
                        y: Some(y),
                    },
                )
            }
            None => Task::none(),
        }
    }

    /// Reset the launcher's sticky state when the active user changes or
    /// transitions to locked. Called from the lock / log-out / user-switch
    /// handlers so the next summon doesn't restore another user's query.
    pub(crate) fn magnify_reset_sticky(&mut self) {
        self.magnify.reset_search();
        self.magnify.anchored_user = self.active_user;
        self.magnify.last_used = None;
    }
}

/// Opens the launcher window hidden at app startup so that subsequent hotkey
/// presses just toggle visibility instead of paying for window creation.
/// `Position::Centered` matches the previous first-summon fallback used when
/// the cursor-monitor lookup is unavailable — every hotkey still calls
/// `magnify_reposition_for_cursor`, so once that stub is implemented per
/// platform the launcher will follow the cursor's monitor on first show.
/// `min_size` / `max_size` bracket the dynamic-resize range so later
/// `window::resize` calls grow / shrink freely without OS clamping. Returns
/// the new id, the initial size (so the caller can register a `WindowInfo`),
/// and the open-task pre-mapped onto `MagnifyMessage::WindowOpened`.
pub(crate) fn open_magnify_window() -> (iced::window::Id, iced::Size, Task<Message>) {
    let height = dims::height_for(0);
    let size = iced::Size::new(dims::WIDTH, height);
    let (id, open_task) = iced::window::open(iced::window::Settings {
        size,
        min_size: Some(iced::Size::new(dims::WIDTH, dims::COLLAPSED_HEIGHT)),
        max_size: Some(iced::Size::new(
            dims::WIDTH,
            dims::height_for(dims::MAX_VISIBLE_ROWS),
        )),
        position: iced::window::Position::Centered,
        resizable: false,
        decorations: false,
        transparent: true,
        level: iced::window::Level::AlwaysOnTop,
        visible: false,
        exit_on_close_request: false,
        platform_specific: magnify_platform_specific(),
        ..Default::default()
    });
    (
        id,
        size,
        open_task.map(|id| Message::Magnify(MagnifyMessage::WindowOpened(id))),
    )
}

/// `move_to` task that puts the launcher's top-left at the cursor monitor's
/// work-area center minus half the window. Returns `Task::none()` when the
/// platform lookup is unavailable so the window stays where it last was.
fn magnify_reposition_for_cursor(id: iced::window::Id, height: f32) -> Task<Message> {
    match cursor_monitor::cursor_monitor_logical_center() {
        Some((cx, cy)) => {
            let top_left = iced::Point::new(cx - dims::WIDTH / 2.0, cy - height / 2.0);
            iced::window::move_to(id, top_left)
        }
        None => Task::none(),
    }
}

/// Per-platform window settings for the launcher. Disable Windows' native
/// rounded corners so the iced-side rounded container we draw isn't doubled
/// (the OS rounding clips the transparent edges differently than our
/// container's border radius).
fn magnify_platform_specific() -> iced::window::settings::PlatformSpecific {
    #[cfg(target_os = "windows")]
    {
        use iced::window::settings::PlatformSpecific;
        PlatformSpecific {
            undecorated_shadow: false,
            corner_preference: iced::window::settings::platform::CornerPreference::DoNotRound,
            skip_taskbar: true,
            ..Default::default()
        }
    }

    #[cfg(target_os = "macos")]
    {
        iced::window::settings::PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        iced::window::settings::PlatformSpecific::default()
    }
}
