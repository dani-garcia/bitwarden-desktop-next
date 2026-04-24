//! `VaultView::view` plus `sheet_view` / `modal_view` / view-internal builders.

use std::sync::Arc;

use bitwarden_vault::CipherListView;
use iced::{
    Alignment, Border, Element, Fill, Length, Padding,
    widget::{Space, column, container, pane_grid, row, text},
};

use crate::{
    components::{
        account_switcher::{self, AccountSwitcherMessage},
        bottom_sheet, buttons, icons, separator_v,
    },
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    SHEET_BREAKPOINT_PX, SHEET_TOP_INSET_PX, SHEET_TOP_RADIUS_PX, VaultMessage,
    state::{PaneKind, VaultView},
    widgets::{
        cipher_form,
        detail_pane::{self, DetailPaneMessage},
        item_list, search_bar, sidebar,
    },
};

impl VaultView {
    pub fn view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Element<'a, VaultMessage, AppTheme> {
        let active_user = ctx.active_user.expect("Screen::Vault without active_user");
        let cached_items: &[Arc<CipherListView>] = self
            .items
            .get(active_user)
            .map(|ic| ic.cached.as_slice())
            .unwrap_or(&[]);

        // --- Sidebar ---
        let sidebar = sidebar::view(&self.sidebar, ctx.colors).map(VaultMessage::Sidebar);

        // --- Content area ---
        // Wide window with detail open  → side-by-side `pane_grid` split.
        // No detail / narrow window     → list fills the area. (For the
        //   narrow case, `App::view_main` overlays the bottom sheet on top
        //   of the entire window — including sidebar and title bar — via
        //   `sheet_view()` below.)
        let show_pane_grid =
            self.selection.detail.is_some() && ctx.window_width >= SHEET_BREAKPOINT_PX;
        let content_area_inner: Element<'a, VaultMessage, AppTheme> = if show_pane_grid {
            pane_grid::PaneGrid::new(
                &self.pane_state,
                move |_pane, kind, _is_maximized| match kind {
                    PaneKind::List => pane_grid::Content::new(self.list_content(cached_items, ctx)),
                    PaneKind::Detail => {
                        let right_pane = self.detail_or_form_pane(ctx.colors, 0.0);
                        let with_separator = row![separator_v(), right_pane].height(Fill);
                        pane_grid::Content::new(with_separator)
                    }
                },
            )
            .on_resize(6, VaultMessage::PaneResized)
            .spacing(1)
            .min_size(250)
            .into()
        } else {
            self.list_content(cached_items, ctx)
        };

        let content_area = container(content_area_inner)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(theme.colors.background)
                    .border(Border::default().rounded(iced::border::top_left(10)))
            });

        let main_row = container(row![sidebar, content_area].height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.header_bg)
            });

        container(main_row)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.background)
            })
            .into()
    }

    /// In narrow mode (`window_width < SHEET_BREAKPOINT_PX`) with a detail
    /// selected, returns the bottom-sheet element that the app composes on
    /// top of the entire window (including sidebar and title bar). Returns
    /// `None` otherwise.
    pub fn sheet_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        if ctx.window_width >= SHEET_BREAKPOINT_PX {
            return None;
        }
        self.selection.detail.as_ref()?;
        let pane = self.detail_or_form_pane(ctx.colors, SHEET_TOP_RADIUS_PX);
        Some(bottom_sheet::view(
            pane,
            SHEET_TOP_INSET_PX,
            Some(VaultMessage::CloseDetailPane),
        ))
    }

    /// Returns the delete-confirmation modal when armed, `None` otherwise.
    /// Composed by the App on top of the vault view so the backdrop covers
    /// the sidebar and title bar.
    pub fn modal_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        if !self.selection.confirm_delete {
            return None;
        }
        let colors = ctx.colors;
        let item_name = self
            .selection
            .detail
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("");

        let title = text(fl!("vault-delete-modal-title"))
            .size(18)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);
        let body = text(fl!("vault-delete-modal-body", name = item_name))
            .size(14)
            .color(colors.text_primary);

        let cancel_btn = buttons::secondary(text(fl!("vault-delete-modal-cancel")).size(14))
            .on_press(VaultMessage::CancelDeleteSelected)
            .padding([8, 20]);
        let confirm_btn = buttons::primary(text(fl!("vault-delete-modal-confirm")).size(14))
            .on_press(VaultMessage::ConfirmDeleteSelected)
            .padding([8, 20]);

        let dialog_inner: Element<'_, VaultMessage, AppTheme> = column![
            title,
            body,
            row![Space::new().width(Fill), cancel_btn, confirm_btn]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(12)
        .padding(Padding::from([16, 20]))
        .width(Length::Fixed(380.0))
        .into();

        let dialog = container(dialog_inner).style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(crate::theme::RADIUS_LG))
        });

        Some(crate::components::modal::view(
            dialog.into(),
            VaultMessage::CancelDeleteSelected,
        ))
    }

    /// Builds the right-side pane content: either the editable `cipher_form`
    /// when `selection.form.is_some()`, or the read-only `detail_pane`.
    fn detail_or_form_pane<'a>(
        &'a self,
        colors: &'a AppColors,
        top_radius: f32,
    ) -> Element<'a, VaultMessage, AppTheme> {
        if let Some(form) = self.selection.form.as_ref() {
            cipher_form::view(form, colors, top_radius).map(VaultMessage::CipherForm)
        } else {
            let item = self
                .selection
                .detail
                .as_ref()
                .expect("detail_or_form_pane called without a selection");
            detail_pane::view(item, colors, top_radius).map(|msg| match msg {
                DetailPaneMessage::Close => VaultMessage::CloseDetailPane,
                other => VaultMessage::DetailPane(other),
            })
        }
    }

    /// Builds the list pane content (header + search + item list).
    fn list_content<'a>(
        &'a self,
        cached_items: &'a [Arc<CipherListView>],
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Element<'a, VaultMessage, AppTheme> {
        let colors = ctx.colors;
        let active_email = ctx
            .active_email
            .expect("Screen::Vault without active_email");
        let title = text(fl!("vault-title"))
            .size(28)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);

        let new_button = buttons::primary(
            row![
                icons::PLUS.render(14.0, colors.card_bg),
                text(fl!("vault-new-button")).size(14),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(VaultMessage::NewItem)
        .padding(Padding {
            top: 8.0,
            right: 16.0,
            bottom: 8.0,
            left: 12.0,
        });

        let account_switcher_open = ctx.open_overlay == Some(crate::app::Overlay::AccountSwitcher);
        let avatar_trigger = account_switcher::avatar_trigger(active_email, colors)
            .map(VaultMessage::AccountSwitcher);
        let dd_panel = account_switcher::dropdown(Some(active_email), ctx.accounts, colors)
            .map(VaultMessage::AccountSwitcher);
        let avatar: Element<'a, VaultMessage, AppTheme> =
            crate::components::drop_down::DropDown::new(
                avatar_trigger,
                dd_panel,
                account_switcher_open,
            )
            .on_dismiss(VaultMessage::AccountSwitcher(
                AccountSwitcherMessage::ToggleDropdown,
            ))
            .alignment(crate::components::drop_down::Alignment::BelowRight)
            .width(360.0)
            .offset(4.0)
            .into();

        let content_header = container(
            row![title, Space::new().width(Fill), new_button, avatar]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .padding([16, 24])
        .width(Fill);

        let search = search_bar::view(&self.search_query).map(VaultMessage::Search);
        let search_row = container(search)
            .padding(Padding {
                top: 0.0,
                right: 24.0,
                bottom: 8.0,
                left: 24.0,
            })
            .width(Fill);

        let item_list = item_list::view(cached_items, self.selection.item, self.list_scroll, ctx)
            .map(VaultMessage::ItemList);

        column![content_header, search_row, item_list]
            .width(Fill)
            .height(Fill)
            .into()
    }
}
