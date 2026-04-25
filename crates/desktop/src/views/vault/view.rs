//! `VaultView::view` plus `sheet_view` / `modal_view` / view-internal builders.

use std::sync::Arc;

use bitwarden_vault::CipherListView;
use iced::{
    Alignment, Border, Element, Fill, Padding,
    widget::{Space, column, container, row, text},
};

use crate::{
    components::{account_switcher, bottom_sheet, buttons, collapsible_pane, icons},
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    SHEET_BREAKPOINT_PX, SHEET_TOP_INSET_PX, SHEET_TOP_RADIUS_PX, VaultMessage,
    state::VaultView,
    widgets::{
        cipher_edit,
        cipher_detail::{self, CipherDetailMessage},
        item_list, search_bar,
    },
};

impl VaultView {
    pub fn view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Element<'a, VaultMessage, AppTheme> {
        let active_user = ctx.active_user.expect("Screen::Vault without active_user");
        let user_cache = self.items.get(active_user);
        let cached_items: &[Arc<CipherListView>] =
            user_cache.map(|ic| ic.cached.as_slice()).unwrap_or(&[]);

        // Wide: collapsible-pane with the detail/form on the right (kept
        // mounted so iced's `pane_grid::diff` never drops child state —
        // see `components::collapsible_pane`).
        // Narrow: bottom sheet overlay composed at App level via
        // `sheet_view`, so the grid drops out entirely here.
        let content_area_inner: Element<'a, VaultMessage, AppTheme> =
            if ctx.window_width >= SHEET_BREAKPOINT_PX {
                let right = self
                    .selection
                    .detail
                    .as_ref()
                    .map(|_| self.detail_or_form_pane(ctx.colors, 0.0));
                collapsible_pane::view(
                    &self.pane,
                    self.list_content(cached_items, ctx),
                    right,
                    VaultMessage::PaneResized,
                )
            } else {
                self.list_content(cached_items, ctx)
            };

        container(content_area_inner)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(theme.colors.background)
                    .border(Border::default().rounded(iced::border::top_left(10)))
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
        let progress = self.selection.sheet_fade.progress_if_visible()?;
        let pane = self.detail_or_form_pane(ctx.colors, SHEET_TOP_RADIUS_PX);
        Some(bottom_sheet::view(
            pane,
            SHEET_TOP_INSET_PX,
            progress,
            Some(VaultMessage::CloseCipherDetail),
        ))
    }

    /// Returns the delete-confirmation modal when armed, `None` otherwise.
    /// Composed by the App on top of the vault view so the backdrop covers
    /// the sidebar and title bar.
    pub fn modal_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        let progress = self.selection.confirm_delete.progress_if_visible()?;
        let colors = ctx.colors;
        let item_name = self
            .selection
            .detail
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("");

        Some(crate::components::modal::confirm_dialog(
            fl!("vault-delete-modal-title"),
            fl!("vault-delete-modal-body", name = item_name),
            fl!("vault-delete-modal-confirm"),
            fl!("vault-delete-modal-cancel"),
            VaultMessage::ConfirmDeleteSelected,
            VaultMessage::CancelDeleteSelected,
            colors,
            progress,
        ))
    }

    /// Builds the right-side pane content: either the editable `cipher_edit`
    /// when `selection.form.is_some()`, or the read-only `cipher_detail`.
    fn detail_or_form_pane<'a>(
        &'a self,
        colors: &'a AppColors,
        top_radius: f32,
    ) -> Element<'a, VaultMessage, AppTheme> {
        if let Some(form) = self.selection.form.as_ref() {
            cipher_edit::view(form, colors, top_radius).map(VaultMessage::CipherEdit)
        } else {
            let item = self
                .selection
                .detail
                .as_ref()
                .expect("detail_or_form_pane called without a selection");
            cipher_detail::view(item, colors, top_radius).map(|msg| match msg {
                CipherDetailMessage::Close => VaultMessage::CloseCipherDetail,
                other => VaultMessage::CipherDetail(other),
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
        let avatar = account_switcher::header_switcher(
            active_email,
            ctx.accounts,
            account_switcher_open,
            colors,
        )
        .map(VaultMessage::AccountSwitcher);

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
