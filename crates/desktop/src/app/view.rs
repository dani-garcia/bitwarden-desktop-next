//! `App::view` + the main-window composition (`view_main`).

use iced::Element;

use crate::{
    components::toast,
    domain::Screen,
    theme::AppTheme,
    views::{title_bar, title_bar::TitleBarMessage},
};

use super::{App, Message, Overlay, RenderCtx, SystemMessage, window::WindowKind};

impl App {
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
            Screen::Login => self.views.login.view(&rctx, server).map(Message::login),
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
                    crate::components::sidebar::view(&self.sidebar, organizations, colors)
                        .map(Message::Sidebar);
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
                .modal_view(&rctx)
                .map(|el| el.map(Message::login)),
            _ => None,
        };

        let settings_modal = self
            .views
            .settings
            .modal_view(&rctx)
            .map(|el| el.map(Message::settings));

        // Both `modal_view` returns `None` when closed so the stack stays
        // cheap (CLAUDE.md → "Stack doesn't cull or clip").
        let generator_modal = self
            .views
            .generator
            .modal_view(&rctx)
            .map(|el| el.map(Message::generator));

        let import_modal = self
            .views
            .import
            .modal_view(&rctx)
            .map(|el| el.map(Message::import));

        let export_modal = self
            .views
            .export
            .modal_view(&rctx)
            .map(|el| el.map(Message::export));

        let new_folder_modal = self
            .views
            .new_folder
            .modal_view(&rctx)
            .map(|el| el.map(Message::new_folder));

        let fingerprint_modal = crate::views::fingerprint_phrase::modal_view(
            &rctx,
            &self.fingerprint,
            Message::System(SystemMessage::CopyFingerprint),
            Message::System(SystemMessage::CloseFingerprintModal),
            Message::System(SystemMessage::OpenLearnMoreFingerprint),
        );

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
}
