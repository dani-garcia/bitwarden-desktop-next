//! `App::view` + the main-window composition (`view_main`).

use iced::Element;

use crate::{
    components::toast,
    domain::Screen,
    theme::AppTheme,
    views::{title_bar, title_bar::TitleBarMessage},
};

use super::{App, Message, Overlay, RenderCtx, SystemMessage, View, ViewExt, window::WindowKind};

impl App {
    pub fn view(&self, window_id: iced::window::Id) -> Element<'_, Message, AppTheme> {
        match self.windows.get(&window_id).map(|w| w.kind) {
            Some(WindowKind::Main) => self.view_main(),
            Some(WindowKind::About) => {
                let rctx = self.render_ctx(window_id);
                crate::views::about::view(&rctx).map(Into::into)
            }
            Some(WindowKind::Magnify) => {
                let rctx = self.render_ctx(window_id);
                crate::views::magnify::view(&self.magnify, &rctx).map(Into::into)
            }
            // Defensive: all windows are inserted at creation, but a stray
            // unknown id renders as empty space rather than panicking.
            None => iced::widget::Space::new().into(),
        }
    }

    /// Build a `RenderCtx` keyed to a specific window's width. Shared between
    /// `view_main` and the About / Magnify secondary windows.
    fn render_ctx(&self, window_id: iced::window::Id) -> RenderCtx<'_> {
        let active = self.active_account_entry();
        let window_width = self
            .windows
            .get(&window_id)
            .map(|info| info.size.width)
            .unwrap_or(1024.0);
        RenderCtx {
            colors: &self.theme.current.colors,
            favicon: &self.favicon,
            show_favicons: self.settings.show_favicons,
            window_width,
            active_user: self.active_user.as_ref(),
            active_email: active.map(|a| a.email.as_str()),
            active_server_url: active.map(|a| a.server_url.as_str()).unwrap_or(""),
            accounts: &self.cache.accounts,
            open_overlay: self.open_overlay,
        }
    }

    fn view_main(&self) -> Element<'_, Message, AppTheme> {
        let rctx = self.render_ctx(self.main_window_id());
        let colors = rctx.colors;

        // Vault / Send return just the right-hand content area — the sidebar
        // is composed below so it persists across screen switches without
        // each view re-rendering it.
        let inner: Element<'_, Message, AppTheme> = match self.screen {
            Screen::Loading => {
                iced::widget::center(crate::components::spinner::spinner(48.0, colors.accent))
                    .into()
            }
            Screen::Login => self.views.login.view(&rctx).map(Into::into),
            Screen::Vault => self.views.vault.view(&rctx).map(Into::into),
            Screen::Send => self.views.send.view(&rctx).map(Into::into),
        };

        let page: Element<'_, Message, AppTheme> =
            if matches!(self.screen, Screen::Vault | Screen::Send) {
                let organizations: &[crate::services::sdk::Organization] = self
                    .active_user
                    .as_ref()
                    .and_then(|uid| self.views.vault.organizations_for(uid))
                    .unwrap_or(&[]);
                let sidebar_el =
                    crate::components::sidebar::view(&self.sidebar, organizations, colors)
                        .map(Into::into);
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

        let mut overlays = self.collect_overlays(&rctx);

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
                .map(Into::into)
        } else {
            title_bar::TitleBarView::view_empty().map(Into::into)
        };

        let main_column: Element<'_, Message, AppTheme> =
            iced::widget::column![tb, page].height(iced::Fill).into();

        // Drag-by-titlebar overlay sits on top when any overlay is up. Only
        // meaningful with the custom title bar — macOS native already handles
        // drag.
        if use_custom_menu_bar && !overlays.is_empty() {
            let drag_strip = iced::widget::mouse_area(
                iced::widget::container(iced::widget::Space::new())
                    .width(iced::Fill)
                    .height(iced::Length::Fixed(title_bar::TITLE_BAR_HEIGHT)),
            )
            .on_press(TitleBarMessage::DragStart.into());
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
            title_bar::resize_wrapper(with_toasts, |dir| TitleBarMessage::ResizeEdge(dir).into())
        } else {
            with_toasts
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

    /// Per-window UI scale fed back to iced. Only the main window honours the
    /// user's zoom setting — the About dialog and Magnify launcher have hand-
    /// tuned fixed layouts and stay at 1.0.
    pub fn scale_factor(&self, window_id: iced::window::Id) -> f32 {
        match self.windows.get(&window_id).map(|w| w.kind) {
            Some(WindowKind::Main) => self.settings.zoom_factor.scale(),
            _ => 1.0,
        }
    }

    /// Build the list of overlays (sub-modals, sheets, app-level modals) that
    /// stack above the active screen this frame, in z-order. The active
    /// screen's `view()` is rendered separately as the page content; its
    /// `overlays()` (sheet + sub-modal) come first, then the global modals
    /// (Settings, Generator, etc.) on top, then the free-function modals
    /// (fingerprint, screenshot confirm).
    fn collect_overlays<'a>(&'a self, rctx: &RenderCtx<'a>) -> Vec<Element<'a, Message, AppTheme>> {
        let mut out: Vec<Element<'a, Message, AppTheme>> = Vec::new();

        // Active screen's sub-overlays. The screen's own `view()` is rendered
        // separately as page content, so only its `overlays()` go here.
        match self.screen {
            Screen::Vault => self.views.vault.push_overlays_into(rctx, &mut out),
            Screen::Send => self.views.send.push_overlays_into(rctx, &mut out),
            Screen::Login => self.views.login.push_overlays_into(rctx, &mut out),
            Screen::Loading => {}
        }

        // App-level modal views. Each renders as an overlay (their `view()`
        // is the modal dialog) plus any nested overlays they own. `Message`
        // is inferred from `out`; each view's local message converts via
        // `impl From<XxxMessage> for Message`.
        self.views.settings.push_into(rctx, &mut out);
        self.views.generator.push_into(rctx, &mut out);
        self.views.import.push_into(rctx, &mut out);
        self.views.export.push_into(rctx, &mut out);
        self.views.new_folder.push_into(rctx, &mut out);

        // Free-function modals — not full `View` impls because their state is
        // held on `App` rather than a dedicated view struct.
        if let Some(el) =
            crate::views::fingerprint_phrase::modal_view(&self.fingerprint, rctx.colors)
        {
            out.push(el.map(Into::into));
        }
        if let Some(el) =
            crate::views::screenshot_confirm::modal_view(&self.screenshot_confirm, rctx.colors)
        {
            out.push(el.map(Into::into));
        }

        out
    }
}
